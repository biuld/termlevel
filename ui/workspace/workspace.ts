import { batch, computed, effect, signal, token } from "nsh";
import { Runtime, Sessions } from "termlevel/native";
import { dropPosition, equalize, geometry, insertPane, minimum, neighbor, nodeIds, panes, removePane, siblings, updateNode, type Direction, type DropPosition, type LayoutNode, type Pane, type Rect, type Size, type TilingState } from "./tiling";
import { groupPalette, type GroupColor } from "../shared/theme";
import { SNAPSHOT_VERSION, type SnapshotPreferences, type WorkspaceSnapshot } from "./workspace-snapshot";

/** Rust owns protocol facts, aggregation and presentation semantics. */
export type TabProgress = import("termlevel/native").ProgressInfo;
export type TabActivity = import("termlevel/native").ActivityInfo;
export type TabStatus = import("termlevel/native").TerminalStatusInfo;

export interface Tab {
  id: string;
  title: string;
  groupId: string | null;
  pinned: boolean;
  cwd: string;
  state?: string;
  exitCode?: number | null;
  error?: string | null;
  inputWarning?: string | null;
  customTitle?: boolean;
  bell?: boolean;
  progress?: TabProgress | null;
  activity?: TabActivity | null;
  status?: TabStatus;
  statusRevision?: number;
  shellPhase?: string;
  /** Latest OSC 7 / ConEmu / iTerm2 working directory. Runtime-only. */
  liveCwd?: string;
  /** OSC 22 pointer shape. Runtime-only. */
  mouseShape?: string;
  /** OSC 133 semantic-prompt state. Runtime-only. */
  atPrompt?: boolean;
}

export interface SessionTransport {
  cwd: string;
  list(): Array<{
    id: string; cwd: string; title: string; state: string; exitCode?: number | null;
    error?: string | null; progress?: TabProgress | null; activity?: TabActivity | null;
    currentCwd?: string | null; mouseShape?: string; atPrompt?: boolean;
    status?: TabStatus; statusRevision?: number; shellPhase?: string;
  }>;
  open(cwd: string): ReturnType<SessionTransport["list"]>[number];
  close(id: string): void;
}

export interface TabGroup {
  id: string;
  title: string;
  cwd: string;
  color: GroupColor;
  collapsed: boolean;
}

/** The directory's own name, which a new group uses as its title. */
function pathName(cwd: string) {
  const trimmed = cwd.trim().replace(/[\\/]+$/, "");
  if (!trimmed) return "未命名分组";
  const cut = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  return (cut >= 0 ? trimmed.slice(cut + 1) : trimmed) || trimmed;
}

/** The home folder reads better as "Home" than as the user's short name. */
function groupName(cwd: string) {
  return cwd === "~" || cwd === Runtime.home() ? "Home" : pathName(cwd);
}

export interface WorkspaceOptions {
  /** The document restored from storage. Null or omitted on a first launch. */
  snapshot?: WorkspaceSnapshot | null;
}

const DEMO_ACTIVE = "prototype";
const DEMO_TABS: Tab[] = [
  { id: "home", title: "Terminal", groupId: null, pinned: true, cwd: "~" },
  { id: "prototype", title: "UI prototype", groupId: "termlevel", pinned: false, cwd: "~/Projects/termlevel" },
  { id: "preview", title: "Build & preview", groupId: "termlevel", pinned: false, cwd: "~/Projects/termlevel" },
  { id: "colors", title: "Color test", groupId: "playground", pinned: false, cwd: "~/Playground" },
];
const DEMO_GROUPS: TabGroup[] = [
  { id: "termlevel", title: "termlevel", cwd: "~/Projects/termlevel", color: "purple", collapsed: false },
  { id: "playground", title: "Playground", cwd: "~/Playground", color: "blue", collapsed: false },
];

/** Trailing number of a generated id, so restored ids never collide with new ones. */
function idSuffix(id: string) {
  const match = /^(?:group|tab|pane|split)-(\d+)$/.exec(id);
  return match ? Number(match[1]) : 0;
}

/** Reopens a saved tab in its directory; one that disappeared falls back to home. */
function openRestored(transport: SessionTransport, cwd: string): ReturnType<SessionTransport["open"]> {
  const requested = cwd || "~";
  const opened = transport.open(requested);
  if (opened.state !== "error" || requested === "~") return opened;
  // The failed entry owns no process; close it so the fallback is the only tab.
  transport.close(opened.id);
  return transport.open("~");
}

export function createWorkspace(transport?: SessionTransport, options: WorkspaceOptions = {}) {
  const snapshot = options.snapshot ?? null;
  // The preview workspace is a deterministic demo; the real app starts empty.
  const demo = !transport && !snapshot;
  const live = transport?.list() ?? [];
  const groups = signal<TabGroup[]>(demo ? DEMO_GROUPS.map((item) => ({ ...item })) : (snapshot?.groups ?? []).map((item) => ({ ...item })));
  const groupIds = new Set(groups().map((item) => item.id));

  // Saved tabs are reopened under new session ids; the old id only keys the layout.
  const remap = new Map<string, string>();
  const initialTabs: Tab[] = [];
  if (snapshot) {
    for (const saved of snapshot.tabs) {
      const existing = live.find((session) => session.id === saved.id);
      const session = existing ?? (transport ? openRestored(transport, saved.cwd) : undefined);
      const id = session?.id ?? saved.id;
      remap.set(saved.id, id);
      const groupId = saved.groupId && groupIds.has(saved.groupId) ? saved.groupId : null;
      initialTabs.push({
        id,
        title: saved.customTitle ? saved.title : session?.title || saved.title,
        cwd: session?.cwd ?? saved.cwd,
        groupId,
        pinned: groupId ? false : saved.pinned,
        customTitle: saved.customTitle,
        state: session?.state,
        exitCode: session?.exitCode,
        error: session?.error,
        progress: session?.progress,
        activity: session?.activity,
        status: session?.status, statusRevision: session?.statusRevision, shellPhase: session?.shellPhase,
        liveCwd: session?.currentCwd ?? undefined,
        mouseShape: session?.mouseShape,
        atPrompt: session?.atPrompt,
      });
    }
  } else if (demo) {
    initialTabs.push(...DEMO_TABS.map((item) => ({ ...item })));
  }
  // A live session the document does not know about (UI hot reload) keeps its tab.
  const known = new Set(initialTabs.map((tab) => tab.id));
  for (const session of live) {
    if (known.has(session.id)) continue;
    known.add(session.id);
    initialTabs.push({ ...session, liveCwd: session.currentCwd ?? undefined, title: session.title || `Terminal ${initialTabs.length + 1}`, groupId: null, pinned: false });
  }
  // IDs, not tab objects or their positions, own the terminal surfaces.
  const tabs = signal<Tab[]>(initialTabs);

  let layoutSequence = 1;
  let sequence = 0;
  if (snapshot) {
    for (const id of [...snapshot.groups.map((item) => item.id), ...snapshot.tabs.map((item) => item.id)]) {
      sequence = Math.max(sequence, idSuffix(id));
    }
    for (const state of Object.values(snapshot.layouts)) {
      for (const id of nodeIds(state.root)) layoutSequence = Math.max(layoutSequence, idSuffix(id));
    }
  }
  // Each tab group is one tiling workspace. Ungrouped and pinned tabs share the "" workspace.
  // The canvas shows only the active workspace, so a layout never mixes groups.
  const NO_LAYOUT: TilingState = { root: null, focusedPaneId: null, zoomed: false };
  const layoutKey = (groupId: string | null | undefined) => groupId ?? "";
  const savedLayouts: Record<string, TilingState> = {};
  if (snapshot) {
    const tabById = new Map(tabs().map((tab) => [tab.id, tab]));
    const remapNode = (node: LayoutNode | null, key: string): LayoutNode | null => {
      if (!node) return null;
      if (node.kind === "pane") {
        const sessionId = remap.get(node.sessionId);
        return sessionId && layoutKey(tabById.get(sessionId)?.groupId) === key ? { ...node, sessionId } : null;
      }
      const first = remapNode(node.first, key), second = remapNode(node.second, key);
      if (!first) return second;
      if (!second) return first;
      return { ...node, first, second };
    };
    for (const [key, state] of Object.entries(snapshot.layouts)) {
      const root = remapNode(state.root, key);
      if (!root) continue;
      const open = panes(root);
      savedLayouts[key] = {
        root,
        focusedPaneId: open.find((pane) => pane.id === state.focusedPaneId)?.id ?? open[0]?.id ?? null,
        zoomed: state.zoomed && open.length > 1,
      };
    }
  }
  const layouts = signal<Record<string, TilingState>>(savedLayouts);
  const activeFromSnapshot = snapshot?.activeTabId ? remap.get(snapshot.activeTabId) : undefined;
  const initialActiveId = snapshot
    ? (activeFromSnapshot && tabs().some((tab) => tab.id === activeFromSnapshot) ? activeFromSnapshot : tabs()[0]?.id ?? null)
    : demo
      ? (tabs().some((tab) => tab.id === DEMO_ACTIVE) ? DEMO_ACTIVE : tabs()[0]?.id ?? null)
      : tabs()[0]?.id ?? null;
  const workspaceKey = signal(layoutKey(tabs().find((tab) => tab.id === initialActiveId)?.groupId));
  // A group's overview replaces the canvas without touching its sessions or layout.
  const dashboardGroups = signal<string[]>(snapshot?.dashboards ?? []);
  const dashboardOpen = computed(() => {
    const key = workspaceKey();
    return key !== "" && dashboardGroups().includes(key);
  });
  // A sidebar choice outside the current split fills the canvas alone. The split tree stays put.
  const foreground = signal<string | null>(null);
  const tiling = computed(() => {
    const id = foreground();
    const current = id ? tabs().find((t) => t.id === id) : undefined;
    const stored = layouts()[workspaceKey()] ?? NO_LAYOUT;
    const open = panes(stored.root);
    if (id && current && layoutKey(current.groupId) === workspaceKey() && open.length >= 2 && !open.some((p) => p.sessionId === id)) {
      const paneId = `solo-${id}`;
      return { root: { kind: "pane" as const, id: paneId, sessionId: id }, focusedPaneId: paneId, zoomed: false };
    }
    return stored;
  });
  const paneList = computed(() => panes(tiling().root));
  const activeId = computed(() => paneList().find(p => p.id === tiling().focusedPaneId)?.sessionId ?? null);
  const inputBlocked = signal(false);
  const cellWidth = signal(0);
  const canvasSize = signal<Size>({ width: 0, height: 0 });
  const paneDrag = signal<{ sessionId: string; targetId: string | null; position: DropPosition; allowed: boolean; joinsGroup: boolean } | null>(null);
  const layoutMessage = signal("");
  const canvasOrigin = signal({ x: 0, y: 0 });
  const dropPreview = signal<Rect | null>(null);
  const resizing = signal(false);
  const recent: string[] = [];
  const nextPane = (sessionId: string): Pane => ({ kind: "pane", id: `pane-${++layoutSequence}`, sessionId });
  const expanded = signal(snapshot?.expanded ?? true);
  const savedWidth = snapshot?.sidebarWidth;
  const sidebarWidth = signal(savedWidth != null ? Math.min(288, Math.max(176, savedWidth)) : token("width-sidebar"));
  const savedFont = snapshot?.fontSize;
  const fontSize = signal(savedFont != null ? Math.min(24, Math.max(10, savedFont)) : token("text-terminal"));
  const clipboardRead = signal(false);
  effect(() => { Sessions.setClipboardRead(clipboardRead()); });
  const scrollStep = Runtime.scrollbackStep();
  const savedScrollback = snapshot?.scrollback;
  const scrollback = signal(savedScrollback != null && scrollStep > 0
    ? Math.min(Runtime.scrollbackMax(), Math.max(Runtime.scrollbackMin(), Math.round(savedScrollback / scrollStep) * scrollStep))
    : Runtime.scrollbackLimit());
  effect(() => { Sessions.setScrollback(scrollback()); });
  const activeTab = computed(() => tabs().find((t) => t.id === activeId()));
  const paneMinimum = computed(() => ({
    width: (cellWidth() || fontSize()) * 40 + token("padding-terminal") * 2,
    height: (fontSize() + token("line-gap-terminal")) * 8 + token("height-toolbar") + token("padding-terminal"),
  }));
  const fullGeometry = computed(() => geometry(tiling().root, { x: 0, y: 0, ...canvasSize() }, paneMinimum(), token("gap-pane")));
  const constrained = computed(() => {
    const size = minimum(tiling().root, paneMinimum(), token("gap-pane"));
    return paneList().length > 1 && (canvasSize().width < size.width || canvasSize().height < size.height);
  });
  const projected = computed(() => {
    if (!tiling().zoomed && !constrained()) return fullGeometry().frames;
    return new Map<string, Rect>(tiling().focusedPaneId ? [[tiling().focusedPaneId!, { x: 0, y: 0, ...canvasSize() }]] : []);
  });
  const paneForSession = (id: string) => paneList().find(p => p.sessionId === id);
  const frameForSession = (id: string) => { const pane = paneForSession(id); return pane ? projected().get(pane.id) : undefined; };
  function setCanvasSize(size: Size) {
    if (size.width !== canvasSize().width || size.height !== canvasSize().height) canvasSize.set(size);
  }

  const tab = (id: string) => tabs().find((t) => t.id === id);
  const group = (id: string) => groups().find((g) => g.id === id);
  const groupTitle = (id: string | null) => id ? group(id)?.title.trim() || "未命名分组" : "Ungrouped";
  const patchTab = (id: string, patch: Partial<Omit<Tab, "id">>) =>
    tabs.update((all) => all.map((t) => t.id === id ? { ...t, ...patch } : t));
  const patchGroup = (id: string, patch: Partial<Omit<TabGroup, "id">>) =>
    groups.update((all) => all.map((g) => g.id === id ? { ...g, ...patch } : g));
  const layoutOf = (key: string) => layouts()[key] ?? NO_LAYOUT;
  function setLayout(key: string, state: TilingState) {
    layouts.update(all => ({ ...all, [key]: state }));
  }
  /** Writes the canvas for the active workspace. A lone sidebar choice must not erase a split. */
  function persistLayout(state: TilingState) {
    if (foreground()) {
      if (panes(layoutOf(workspaceKey()).root).length >= 2) {
        layoutMessage.set("That group already has a split.");
        return false;
      }
      foreground.set(null);
    }
    setLayout(workspaceKey(), state);
    return true;
  }
  function pruneLayout(key: string, ids: Set<string>) {
    const state = layoutOf(key);
    if (!state.root) return;
    const removed = panes(state.root).filter(p => ids.has(p.sessionId));
    if (!removed.length) return;
    const previous = state.root;
    const focusedGone = removed.some(p => p.id === state.focusedPaneId);
    let root: LayoutNode | null = state.root;
    for (const pane of removed) root = removePane(root, pane.id);
    const open = panes(root);
    let focus = open.find(p => p.id === state.focusedPaneId)?.id ?? null;
    if (!focus && focusedGone) {
      focus = siblings(previous, state.focusedPaneId ?? "").find(p => open.some(q => q.id === p.id))?.id ?? open[0]?.id ?? null;
    }
    setLayout(key, { root, focusedPaneId: focus ?? open[0]?.id ?? null, zoomed: root ? state.zoomed : false });
  }
  function forgetSession(id: string) {
    for (const key of Object.keys(layouts())) {
      if (panes(layoutOf(key).root).some(p => p.sessionId === id)) pruneLayout(key, new Set([id]));
    }
  }

  function closeDashboardFor(groupId: string | null | undefined) {
    const key = layoutKey(groupId);
    if (!key) return;
    dashboardGroups.update(all => all.filter(id => id !== key));
  }

  function openDashboard(groupId: string) {
    if (!group(groupId)) return;
    batch(() => {
      workspaceKey.set(groupId);
      patchGroup(groupId, { collapsed: false });
      dashboardGroups.update(all => all.includes(groupId) ? all : [...all, groupId]);
    });
  }

  function closeDashboard() {
    const key = workspaceKey();
    if (key) closeDashboardFor(key);
  }

  function select(id: string, acknowledge = true) {
    if (!tab(id)) return;
    const observedRevision = tab(id)?.statusRevision;
    batch(() => {
      closeDashboardFor(tab(id)?.groupId);
      const key = layoutKey(tab(id)?.groupId);
      if (workspaceKey() !== key) workspaceKey.set(key);
      const state = layoutOf(key);
      const open = panes(state.root);
      const existing = open.find(p => p.sessionId === id);
      if (existing) {
        foreground.set(null);
        if (state.focusedPaneId !== existing.id) setLayout(key, { ...state, focusedPaneId: existing.id });
      } else if (open.length >= 2) {
        // The split stays intact. The chosen terminal takes the whole canvas.
        foreground.set(id);
      } else {
        foreground.set(null);
        const focused = open.find(p => p.id === state.focusedPaneId) ?? open[0];
        if (state.root && focused) {
          setLayout(key, { ...state, root: updateNode(state.root, focused.id, () => ({ ...focused, sessionId: id })), focusedPaneId: focused.id });
        } else {
          const pane = nextPane(id);
          setLayout(key, { root: pane, focusedPaneId: pane.id, zoomed: false });
        }
      }
      const old = recent.indexOf(id); if (old >= 0) recent.splice(old, 1); recent.unshift(id);
      if (tab(id)?.bell) patchTab(id, { bell: false });
      const groupId = tab(id)?.groupId;
      if (groupId) patchGroup(groupId, { collapsed: false });
    });
    if (acknowledge && observedRevision != null) Sessions.acknowledge(id, observedRevision);
  }

  function addTab(groupId: string | null = null, focus = true) {
    sequence++;
    const destination = groupId ? group(groupId) : undefined;
    // With no active terminal, a new one starts in the home folder.
    const cwd = destination?.cwd || activeTab()?.liveCwd || activeTab()?.cwd || "~";
    const session = transport?.open(cwd);
    const id = session?.id ?? `tab-${sequence}`;
    const validGroup = destination ? groupId : null;
    batch(() => {
      closeDashboardFor(validGroup);
      tabs.update((all) => [...all, {
        ...session, id, title: session?.title || `Terminal ${sequence}`, groupId: validGroup, pinned: false,
        cwd: session?.cwd ?? cwd, liveCwd: session?.currentCwd ?? undefined,
        mouseShape: session?.mouseShape, atPrompt: session?.atPrompt,
        status: session?.status, statusRevision: session?.statusRevision, shellPhase: session?.shellPhase,
      }]);
      if (focus) select(id);
    });
    return id;
  }

  function focusPane(id: string) { const pane = paneList().find(p => p.id === id); if (pane) select(pane.sessionId); }
  function canSplit(direction: Direction, targetId = tiling().focusedPaneId) {
    const box = targetId ? fullGeometry().frames.get(targetId) : null;
    if (!box) return false;
    const min = paneMinimum(), gap = token("gap-pane");
    return !constrained() && (direction === "left" || direction === "right"
      ? box.width >= min.width * 2 + gap && box.height >= min.height
      : box.height >= min.height * 2 + gap && box.width >= min.width);
  }
  function splitTerminal(direction: Direction, existingId?: string) {
    if (existingId && !tab(existingId)) return false;
    // A terminal from another group keeps that group's layout. Dragging it onto
    // this canvas is what joins the group; the menu only switches workspace.
    if (existingId && layoutKey(tab(existingId)?.groupId) !== workspaceKey()) { select(existingId); return true; }
    if (existingId && (paneForSession(existingId) || panes(layoutOf(workspaceKey()).root).some(p => p.sessionId === existingId))) {
      select(existingId); return true;
    }
    if (foreground() && panes(layoutOf(workspaceKey()).root).length >= 2) {
      layoutMessage.set("That group already has a split.");
      return false;
    }
    if (!canSplit(direction)) { layoutMessage.set("Not enough room to split. Replace this terminal or hide another pane."); return false; }
    const state = tiling();
    if (!state.root || !state.focusedPaneId) return false;
    batch(() => {
      const id = existingId ?? addTab(activeTab()?.groupId ?? null, false);
      const pane = nextPane(id);
      if (!persistLayout({ root: insertPane(state.root!, state.focusedPaneId!, pane, direction, `split-${++layoutSequence}`), focusedPaneId: pane.id, zoomed: false })) return;
      layoutMessage.set(""); select(id);
    });
    return true;
  }
  /** Sessions currently placed as panes in one group's layout, in tree order. */
  function layoutSessions(groupId: string | null) {
    return panes(layoutOf(layoutKey(groupId)).root).map(p => p.sessionId);
  }
  /** Moves a whole split, pane for pane, into a tab group or out to stand alone. */
  function placeSplit(from: string | null, to: string | null) {
    const fromKey = layoutKey(from), toKey = layoutKey(to);
    if (fromKey === toKey) return;
    if (to && !group(to)) return;
    const state = layoutOf(fromKey);
    const ids = panes(state.root).map(p => p.sessionId);
    if (ids.length < 2) return;
    if (panes(layoutOf(toKey).root).length >= 2) {
      layoutMessage.set("That group already has a split.");
      return;
    }
    const viewing = workspaceKey() === fromKey;
    batch(() => {
      tabs.update(all => all.map(t => ids.includes(t.id) ? { ...t, groupId: to, pinned: to ? false : t.pinned } : t));
      layouts.update(all => {
        const copy: Record<string, TilingState> = { ...all, [toKey]: state };
        delete copy[fromKey];
        return copy;
      });
      if (to) patchGroup(to, { collapsed: false });
      if (viewing) workspaceKey.set(toKey);
      layoutMessage.set("");
    });
  }
  function closeTabs(ids: string[]) {
    const valid = ids.filter(id => !!tab(id));
    if (!valid.length) return;
    for (const id of valid) transport?.close(id);
    batch(() => {
      const closing = new Set(valid);
      const keys = new Set(valid.map(id => layoutKey(tab(id)?.groupId)));
      tabs.update(all => all.filter(t => !closing.has(t.id)));
      for (const key of keys) pruneLayout(key, closing);
      const active = layoutOf(workspaceKey());
      const still = panes(active.root);
      if (still.length) {
        const focus = still.find(p => p.id === active.focusedPaneId) ?? still[0];
        select(focus.sessionId);
        return;
      }
      const key = workspaceKey();
      const sibling = recent.find(id => tab(id) && layoutKey(tab(id)?.groupId) === key)
        ?? tabs().find(t => layoutKey(t.groupId) === key)?.id;
      if (sibling) { select(sibling); return; }
      const fallback = recent.find(id => !!tab(id)) ?? tabs()[0]?.id;
      if (fallback) select(fallback);
      else { foreground.set(null); workspaceKey.set(""); }
    });
  }
  function closeTab(id: string) { closeTabs([id]); }
  /** A shell that exits 0, including Ctrl+D on an empty line, leaves no pane behind. */
  function closeCleanExit(id: string) { closeTab(id); }
  function hidePane(id = tiling().focusedPaneId) {
    if (paneList().length <= 1 || !id) return;
    const pane = paneList().find(p => p.id === id); if (!pane) return;
    batch(() => {
      pruneLayout(workspaceKey(), new Set([pane.sessionId]));
      const state = layoutOf(workspaceKey());
      setLayout(workspaceKey(), { ...state, zoomed: false });
      const focus = panes(state.root).find(p => p.id === state.focusedPaneId);
      if (focus) select(focus.sessionId);
    });
  }
  function focusDirection(direction: Direction, swap = false) {
    const state = tiling(); if (!state.focusedPaneId || !state.root) return;
    const other = neighbor(state.focusedPaneId, direction, fullGeometry().frames); if (!other) return;
    if (!swap) { focusPane(other); return; }
    const a = paneList().find(p => p.id === state.focusedPaneId)!, b = paneList().find(p => p.id === other)!;
    const root = updateNode(updateNode(state.root, a.id, () => ({ ...a, sessionId: b.sessionId })), b.id, () => ({ ...b, sessionId: a.sessionId }));
    persistLayout({ ...state, root, focusedPaneId: b.id });
  }
  function resizeSplit(id: string, ratio: number) {
    const limit = fullGeometry().dividers.find(d => d.id === id);
    if (!limit || !Number.isFinite(ratio) || limit.minRatio > limit.maxRatio) return;
    const value = Math.max(limit.minRatio, Math.min(limit.maxRatio, ratio));
    const state = tiling();
    persistLayout({ ...state, root: state.root ? updateNode(state.root, id, node => node.kind === "split" ? { ...node, ratio: value } : node) : null });
  }
  function resizeDirection(direction: Direction) {
    const focus = tiling().focusedPaneId, box = focus ? fullGeometry().frames.get(focus) : null;
    if (!box) return;
    const row = direction === "left" || direction === "right";
    const candidates = fullGeometry().dividers.filter(d => d.axis === (row ? "row" : "column") && box.x >= d.parent.x && box.y >= d.parent.y && box.x + box.width <= d.parent.x + d.parent.width + 1 && box.y + box.height <= d.parent.y + d.parent.height + 1);
    const divider = candidates[candidates.length - 1]; if (!divider) return;
    const extent = (row ? divider.parent.width : divider.parent.height) - token("gap-pane");
    const ratio = row ? (divider.rect.x - divider.parent.x) / extent : (divider.rect.y - divider.parent.y) / extent;
    resizeSplit(divider.id, ratio + (direction === "right" || direction === "down" ? 0.04 : -0.04));
  }
  function toggleZoom() { if (paneList().length > 1) persistLayout({ ...tiling(), zoomed: !tiling().zoomed }); }
  function equalizeLayout() { const state = tiling(); persistLayout({ ...state, root: state.root ? equalize(state.root) : null }); }
  function dropLayout(sessionId: string, targetId: string, position: DropPosition, commit = true) {
    if (foreground() && panes(layoutOf(workspaceKey()).root).length >= 2) {
      const open = panes(layoutOf(workspaceKey()).root);
      if (open.some(p => p.sessionId === sessionId) || position === "center") {
        if (commit) select(sessionId);
        else dropPreview.set({ x: 0, y: 0, ...canvasSize() });
        return true;
      }
      if (commit) layoutMessage.set("That group already has a split.");
      return false;
    }
    const state = tiling(), target = paneList().find(p => p.id === targetId), source = paneForSession(sessionId);
    if (!state.root || !target || !tab(sessionId) || source?.id === targetId || state.zoomed || constrained()) return false;
    const joins = layoutKey(tab(sessionId)?.groupId) !== workspaceKey();
    let root = state.root, focus = target.id;
    if (position === "center") {
      root = updateNode(root, target.id, () => ({ ...target, sessionId }));
      if (source) root = updateNode(root, source.id, () => ({ ...source, sessionId: target.sessionId }));
    } else {
      if (source) root = removePane(root, source.id)!;
      const added = source ?? { kind: "pane" as const, id: `pane-${layoutSequence + 1}`, sessionId };
      root = insertPane(root, target.id, added, position, `split-${layoutSequence + 2}`);
      const min = minimum(root, paneMinimum(), token("gap-pane"));
      if (min.width > canvasSize().width || min.height > canvasSize().height) return false;
      focus = added.id;
      if (commit) layoutSequence += 2;
    }
    if (commit) {
      batch(() => {
        if (joins) {
          const groupId = workspaceKey() || null;
          patchTab(sessionId, { groupId, pinned: groupId ? false : tab(sessionId)?.pinned });
          forgetSession(sessionId);
        }
        if (!persistLayout({ root, focusedPaneId: focus, zoomed: false })) return;
        select(sessionId);
      });
    } else dropPreview.set(geometry(root, { x: 0, y: 0, ...canvasSize() }, paneMinimum(), token("gap-pane")).frames.get(focus) ?? null);
    return true;
  }
  function beginPaneDrag(sessionId: string) {
    if (constrained()) return;
    if (tiling().zoomed) persistLayout({ ...tiling(), zoomed: false });
    paneDrag.set({ sessionId, targetId: null, position: "center", allowed: false, joinsGroup: layoutKey(tab(sessionId)?.groupId) !== workspaceKey() });
  }
  function trackPaneDrag(pageX: number, pageY: number) {
    const drag = paneDrag(); if (!drag) return;
    const x = pageX - canvasOrigin().x, y = pageY - canvasOrigin().y;
    const target = [...projected()].find(([, r]) => x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height);
    if (!target) { paneDrag.set({ ...drag, targetId: null, allowed: false }); dropPreview.set(null); return; }
    const position = dropPosition(target[1], x, y);
    const allowed = dropLayout(drag.sessionId, target[0], position, false);
    if (!allowed) dropPreview.set(target[1]);
    paneDrag.set({ ...drag, targetId: target[0], position, allowed });
  }
  function finishPaneDrag() {
    const drag = paneDrag();
    const applied = !!drag?.targetId && drag.allowed && dropLayout(drag.sessionId, drag.targetId, drag.position);
    cancelPaneDrag(); return applied;
  }
  function cancelPaneDrag() { paneDrag.set(null); dropPreview.set(null); }

  /** A palette color not already on a group, or any color once all nine are used. */
  function nextGroupColor(): GroupColor {
    const used = new Set(groups().map((item) => item.color));
    const open = groupPalette.filter((item) => !used.has(item.id));
    const pool = open.length > 0 ? open : [...groupPalette];
    return pool[Math.floor(Math.random() * pool.length)].id;
  }

  function addGroup(cwd: string) {
    const directory = cwd.trim() || "~";
    const id = `group-${++sequence}`;
    const color = nextGroupColor();
    batch(() => {
      groups.update((all) => [...all, {
        id, title: groupName(directory), cwd: directory, color, collapsed: false,
      }]);
      expanded.set(true);
    });
    addTab(id);
    return id;
  }

  function ungroup(id: string) {
    const members = tabs().filter(t => t.groupId === id);
    const viewing = workspaceKey() === id;
    const groupLayout = layoutOf(id);
    const loose = layoutOf("");
    const kept = panes(groupLayout.root).find(p => p.id === groupLayout.focusedPaneId)?.sessionId ?? members[0]?.id;
    batch(() => {
      closeDashboardFor(id);
      tabs.update((all) => all.map((t) => t.groupId === id ? { ...t, groupId: null } : t));
      groups.update((all) => all.filter((g) => g.id !== id));
      // An empty ungrouped workspace inherits the group's arrangement. A workspace
      // that already has panes keeps it; the terminal on screen is placed into it.
      const adopted = !loose.root && groupLayout.root ? groupLayout : loose;
      layouts.update(all => {
        const copy: Record<string, TilingState> = { ...all, "": adopted };
        delete copy[id];
        return copy;
      });
      if (viewing && kept) {
        workspaceKey.set("");
        select(kept);
      }
    });
  }

  function closeGroup(id: string) {
    batch(() => {
      closeDashboardFor(id);
      closeTabs(tabs().filter(t => t.groupId === id).map(t => t.id));
      groups.update((all) => all.filter((g) => g.id !== id));
      layouts.update(all => {
        if (!(id in all)) return all;
        const copy = { ...all };
        delete copy[id];
        return copy;
      });
    });
  }

  /** Moves `id` into a section without changing its session id.
   * `beforeId` is the tab it should precede; null appends to that section.
   * A group destination expands the group and clears the pin. */
  function placeTab(id: string, destination: { groupId: string | null; pinned: boolean; beforeId: string | null }) {
    const current = tab(id);
    if (!current) return;
    if (destination.beforeId === id && destination.groupId === current.groupId && destination.pinned === current.pinned) return;
    if (destination.groupId && !group(destination.groupId)) return;
    const pinned = destination.groupId ? false : destination.pinned;
    const groupId = pinned ? null : destination.groupId;
    const from = tabs().findIndex((t) => t.id === id);
    const next = tabs().filter((t) => t.id !== id);
    let index = next.length;
    if (destination.beforeId && destination.beforeId !== id) {
      const at = next.findIndex((t) => t.id === destination.beforeId);
      if (at < 0) return;
      index = at;
    } else {
      let last = -1;
      next.forEach((t, i) => {
        if (t.groupId === groupId && t.pinned === pinned) last = i;
      });
      if (last >= 0) index = last + 1;
    }
    if (current.groupId === groupId && current.pinned === pinned && from === index) return;
    next.splice(index, 0, { ...current, groupId, pinned });
    const previousKey = layoutKey(current.groupId);
    const nextKey = layoutKey(groupId);
    const follow = activeId() === id;
    batch(() => {
      tabs.set(next);
      if (groupId) patchGroup(groupId, { collapsed: false });
      if (previousKey !== nextKey) {
        forgetSession(id);
        if (follow) select(id);
      }
    });
  }

  function inSplit(id: string) {
    const current = tab(id);
    if (!current) return false;
    const ids = layoutSessions(current.groupId);
    return ids.length >= 2 && ids.includes(id);
  }
  function moveTab(id: string, destination: string | null) {
    const current = tab(id);
    if (current && inSplit(id)) {
      placeSplit(current.groupId, destination);
      return;
    }
    placeTab(id, { groupId: destination, pinned: false, beforeId: null });
  }

  /** Drops `sourceId` on a tab that is not already in a group.
   * Both land in a new group, `sourceId` before the target when `before` is set.
   * Session ids stay the same, and pins are cleared. */
  function combineTabs(sourceId: string, targetId: string, before: boolean) {
    const source = tab(sourceId);
    const target = tab(targetId);
    if (!source || !target || sourceId === targetId || target.groupId) return null;
    const groupId = `group-${++sequence}`;
    const ordered = (before ? [source, target] : [target, source]).map((item) => ({ ...item, groupId, pinned: false }));
    const next = tabs().filter((item) => item.id !== sourceId && item.id !== targetId);
    const targetIndex = tabs().findIndex((item) => item.id === targetId);
    const sourceWasBefore = tabs().findIndex((item) => item.id === sourceId) < targetIndex;
    next.splice(Math.max(0, targetIndex - (sourceWasBefore ? 1 : 0)), 0, ...ordered);
    const cwd = target.cwd || "~";
    const color = nextGroupColor();
    const follow = activeId() === sourceId || activeId() === targetId;
    const shown = follow ? activeId()! : targetId;
    batch(() => {
      groups.update((all) => [...all, { id: groupId, title: groupName(cwd), cwd, color, collapsed: false }]);
      tabs.set(next);
      expanded.set(true);
      forgetSession(sourceId);
      forgetSession(targetId);
      const pane = nextPane(shown);
      setLayout(groupId, { root: pane, focusedPaneId: pane.id, zoomed: false });
      if (follow) select(shown);
    });
    return groupId;
  }

  function togglePin(id: string) {
    const current = tab(id);
    if (!current) return;
    placeTab(id, { groupId: null, pinned: !current.pinned, beforeId: null });
  }

  function step(direction: number) {
    const all = tabs();
    if (!all.length) return;
    const index = all.findIndex((t) => t.id === activeId());
    select(all[(index + direction + all.length) % all.length].id);
  }

  function reorder(id: string, direction: number) {
    const current = tab(id);
    if (!current) return;
    const all = [...tabs()];
    const peers = all.filter((t) => t.groupId === current.groupId && t.pinned === current.pinned);
    const target = peers[peers.findIndex((t) => t.id === id) + direction];
    if (!target) return;
    const a = all.findIndex((t) => t.id === id);
    const b = all.findIndex((t) => t.id === target.id);
    [all[a], all[b]] = [all[b], all[a]];
    tabs.set(all);
  }

  /** The serializable workspace. Transient session state, bells and processes stay out. */
  function snapshotFile(preferences: SnapshotPreferences): WorkspaceSnapshot {
    const open: Record<string, TilingState> = {};
    for (const [key, state] of Object.entries(layouts())) {
      if (state.root) open[key] = { root: state.root, focusedPaneId: state.focusedPaneId, zoomed: state.zoomed };
    }
    return {
      version: SNAPSHOT_VERSION,
      groups: groups().map(({ id, title, cwd, color, collapsed }) => ({ id, title, cwd, color, collapsed })),
      tabs: tabs().map(({ id, title, cwd, liveCwd, groupId, pinned, customTitle }) => ({
        id, title, cwd: liveCwd ?? cwd, groupId, pinned, customTitle: !!customTitle,
      })),
      layouts: open,
      activeTabId: activeId(),
      workspaceKey: workspaceKey(),
      dashboards: dashboardGroups().filter(id => groups().some(g => g.id === id)),
      expanded: expanded(),
      sidebarWidth: sidebarWidth(),
      fontSize: fontSize(),
      scrollback: scrollback(),
      ...preferences,
    };
  }

  // Restoring is not a user selection: keep the saved focus and collapsed groups,
  // and do not acknowledge program status the user has not seen.
  if (initialActiveId) select(initialActiveId, false);
  if (snapshot) {
    // Restoring the saved overview happens after the initial selection, which
    // otherwise counts as choosing a terminal and closes it.
    dashboardGroups.set(snapshot.dashboards.filter(id => !!group(id)));
    for (const saved of snapshot.groups) {
      if (saved.collapsed !== group(saved.id)?.collapsed) patchGroup(saved.id, { collapsed: saved.collapsed });
    }
  }

  return {
    tabs, groups, activeId, activeTab, expanded, sidebarWidth, fontSize, scrollback, clipboardRead, workspaceKey, snapshot: snapshotFile,
    dashboardOpen, openDashboard, closeDashboard,
    tiling, paneList, inputBlocked, cellWidth, canvasSize, setCanvasSize, paneMinimum, fullGeometry, constrained, projected,
    paneForSession, frameForSession, focusPane, canSplit, splitTerminal, hidePane, toggleZoom, focusDirection,
    layoutSessions, placeSplit, inSplit,
    resizeSplit, resizeDirection, equalizeLayout, paneDrag, dropLayout, layoutMessage, resizing, canvasOrigin, dropPreview, beginPaneDrag, trackPaneDrag, finishPaneDrag, cancelPaneDrag,
    tab, group, groupTitle, patchTab, patchGroup, select, addTab, closeTab, closeCleanExit, addGroup,
    ungroup, closeGroup, moveTab, placeTab, combineTabs, togglePin, step, reorder,
  };
}

export type Workspace = ReturnType<typeof createWorkspace>;
