import { signal } from "nsh";
import { Runtime } from "termlevel/native";
import { nodeIds, panes, type LayoutNode, type TilingState } from "../tiling";
import type { SessionTransport, Tab, TabGroup, WorkspaceOptions } from "./types";

/** The directory's own name, which a new group uses as its title. */
function pathName(cwd: string) {
  const trimmed = cwd.trim().replace(/[\\/]+$/, "");
  if (!trimmed) return "未命名分组";
  const cut = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  return (cut >= 0 ? trimmed.slice(cut + 1) : trimmed) || trimmed;
}

/** The home folder reads better as "Home" than as the user's short name. */
export function groupName(cwd: string) {
  return cwd === "~" || cwd === Runtime.home() ? "Home" : pathName(cwd);
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

export function initialWorkspace(transport?: SessionTransport, options: WorkspaceOptions = {}) {
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
        pinned: saved.pinned,
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
  return { snapshot, groups, tabs, layoutSequence, sequence, NO_LAYOUT, layoutKey, layouts, initialActiveId, workspaceKey };
}
