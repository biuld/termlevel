import { batch, computed, effect, signal, token } from "nsh";
import { Runtime, Sessions } from "termlevel/native";
import { dropPosition, equalize, geometry, insertPane, minimum, neighbor, panes, removePane, siblings, updateNode, type Direction, type DropPosition, type LayoutNode, type Pane, type Rect, type Size, type TilingState } from "../tiling";
import { SNAPSHOT_VERSION, type GitView, type SnapshotPreferences, type WorkspaceSnapshot } from "../workspace-snapshot";
import { createRepositoryPreferences } from "../repository-preferences";
import { createFiles } from "./files";
import { initialWorkspace } from "./initial";
import { groupOperations } from "./groups";
import type { SessionTransport, Tab, TabGroup, WorkspaceOptions } from "./types";
export type { SessionTransport, Tab, TabActivity, TabGroup, TabProgress, TabStatus, WorkspaceOptions } from "./types";

export function createWorkspace(transport?: SessionTransport, options: WorkspaceOptions = {}) {
  const { snapshot, groups, tabs, NO_LAYOUT, layoutKey, layouts, initialActiveId, workspaceKey, ...counters } = initialWorkspace(transport, options);
  let { sequence, layoutSequence } = counters;
  const savedGitViews = snapshot?.gitViews ?? Object.fromEntries((snapshot?.dashboards ?? []).map(id => [id, { open: ["changes" as const], selected: "changes" as const }]));
  const gitViews = signal<Record<string, { open: GitView[]; selected: GitView | null }>>(savedGitViews);
  const repositories = createRepositoryPreferences(snapshot?.gitRepositories ?? {}, () => groups().map(group => group.id));
  const savedFiles = snapshot?.files ?? {};
  const fileViews = createFiles({});
  const activeGitView = computed(() => gitViews()[workspaceKey()]?.selected ?? null);
  const filesShowing = computed(() => !!fileViews.files()[workspaceKey()]?.showing);
  const canvasBlocked = computed(() => activeGitView() !== null || filesShowing());
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

  function selectGroup(id: string) {
    if (!group(id)) return;
    batch(() => {
      workspaceKey.set(id);
      patchGroup(id, { collapsed: false });
      if (!gitViews()[id]?.selected && !layoutOf(id).root) {
        const first = tabs().find(item => item.groupId === id);
        if (first) select(first.id);
      }
    });
  }

  function selectUngrouped() {
    batch(() => {
      workspaceKey.set("");
      if (!layoutOf("").root) {
        const first = tabs().find(item => !item.groupId);
        if (first) select(first.id);
      }
    });
  }

  function openGitView(kind: GitView, id = workspaceKey()) {
    if (!group(id)) return;
    batch(() => {
      selectGroup(id);
      fileViews.hide(id);
      gitViews.update(all => {
        const old = all[id] ?? { open: [], selected: null };
        return { ...all, [id]: { open: old.open.includes(kind) ? old.open : [...old.open, kind], selected: kind } };
      });
    });
  }

  function openFiles(id = workspaceKey()) {
    if (!group(id)) return;
    batch(() => {
      selectGroup(id);
      const old = gitViews()[id];
      if (old?.selected) gitViews.update(all => ({ ...all, [id]: { ...old, selected: null } }));
      fileViews.show(id);
    });
  }

  function closeFiles(id: string) {
    fileViews.close(id);
  }

  function closeGitView(id: string, kind: GitView) {
    const old = gitViews()[id];
    if (!old || !old.open.includes(kind)) return;
    const open = old.open.filter(view => view !== kind);
    gitViews.update(all => ({ ...all, [id]: { open, selected: old.selected === kind ? null : old.selected } }));
  }

  function select(id: string, acknowledge = true) {
    if (!tab(id)) return;
    const observedRevision = tab(id)?.statusRevision;
    batch(() => {
      const key = layoutKey(tab(id)?.groupId);
      if (key && gitViews()[key]?.selected) gitViews.update(all => ({ ...all, [key]: { ...all[key], selected: null } }));
      if (key) fileViews.hide(key);
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

  function addTab(groupId: string | null = null, focus = true, directory = "") {
    sequence++;
    const destination = groupId ? group(groupId) : undefined;
    // With no active terminal, a new one starts in the home folder.
    const cwd = directory || destination?.cwd || activeTab()?.liveCwd || activeTab()?.cwd || "~";
    const session = transport?.open(cwd);
    const id = session?.id ?? `tab-${sequence}`;
    const validGroup = destination ? groupId : null;
    batch(() => {
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
      tabs.update(all => all.map(t => ids.includes(t.id) ? { ...t, groupId: to } : t));
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
      // A background shell exiting must not replace the selected Git view.
      if (activeGitView()) return;
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
          patchTab(sessionId, { groupId });
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

  const { addGroup, ungroup, closeGroup, placeTab, inSplit, moveTab, combineTabs, togglePin, step, reorder } = groupOperations({
    groups, tabs, expanded, workspaceKey, layouts, gitViews, activeId, nextSequence: () => ++sequence,
    tab, group, addTab, closeTabs, layoutOf, layoutKey, layoutSessions, placeSplit,
    select, forgetSession, patchGroup, nextPane, setLayout, forgetFiles: fileViews.forget,
  });

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
      dashboards: [],
      gitRepositories: Object.fromEntries(Object.entries(repositories.repositoryPreferences()).filter(([id]) => !!group(id))),
      gitViews: Object.fromEntries(Object.entries(gitViews()).filter(([id]) => !!group(id))),
      files: Object.fromEntries(Object.entries(fileViews.files()).filter(([id]) => !!group(id))),
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
    // Restoring saved Git views happens after the initial selection, which
    // otherwise counts as choosing a terminal and closes it.
    gitViews.set(savedGitViews);
    fileViews.files.set(savedFiles);
    if (!snapshot.workspaceKey || group(snapshot.workspaceKey)) workspaceKey.set(snapshot.workspaceKey);
    for (const saved of snapshot.groups) {
      if (saved.collapsed !== group(saved.id)?.collapsed) patchGroup(saved.id, { collapsed: saved.collapsed });
    }
  }

  return {
    tabs, groups, activeId, activeTab, expanded, sidebarWidth, fontSize, scrollback, clipboardRead, workspaceKey, snapshot: snapshotFile,
    ...repositories, gitViews, activeGitView, files: fileViews.files, filesShowing, openFiles, closeFiles,
    patchFiles: fileViews.patch, canvasBlocked, selectGroup, selectUngrouped, openGitView, closeGitView,
    tiling, paneList, inputBlocked, cellWidth, canvasSize, setCanvasSize, paneMinimum, fullGeometry, constrained, projected,
    paneForSession, frameForSession, focusPane, canSplit, splitTerminal, hidePane, toggleZoom, focusDirection,
    layoutSessions, placeSplit, inSplit,
    resizeSplit, resizeDirection, equalizeLayout, paneDrag, dropLayout, layoutMessage, resizing, canvasOrigin, dropPreview, beginPaneDrag, trackPaneDrag, finishPaneDrag, cancelPaneDrag,
    tab, group, groupTitle, patchTab, patchGroup, select, addTab, closeTab, closeCleanExit, addGroup,
    ungroup, closeGroup, moveTab, placeTab, combineTabs, togglePin, step, reorder,
  };
}

export type Workspace = ReturnType<typeof createWorkspace>;
