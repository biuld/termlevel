import { groupColorIds, type GroupColor } from "./theme";
import { nodeIds, panes, type LayoutNode, type TilingState } from "./tiling";

/**
 * The persisted workspace structure. Sessions are not part of it: a tab keeps
 * its initial directory and title, and the next launch starts a fresh shell.
 * `version` changes whenever the shape does; older documents are discarded.
 */
export const SNAPSHOT_VERSION = 1;

export type Appearance = "system" | "light" | "dark";

export interface PersistedTab {
  id: string;
  title: string;
  cwd: string;
  groupId: string | null;
  pinned: boolean;
  customTitle: boolean;
}

export interface PersistedGroup {
  id: string;
  title: string;
  cwd: string;
  color: GroupColor;
  collapsed: boolean;
}

/** Settings that live outside the workspace signals but belong in the same document. */
export interface SnapshotPreferences {
  appearance: Appearance;
  interfaceFont: string;
  monoFont: string;
}

export interface WorkspaceSnapshot {
  version: number;
  groups: PersistedGroup[];
  tabs: PersistedTab[];
  layouts: Record<string, TilingState>;
  activeTabId: string | null;
  workspaceKey: string;
  expanded: boolean;
  sidebarWidth: number | null;
  fontSize: number | null;
  scrollback: number | null;
  appearance: Appearance;
  /** Null when the stored family is absent or not a known choice. */
  interfaceFont: string | null;
  monoFont: string | null;
}

const appearances: readonly string[] = ["system", "light", "dark"];

function object(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function nonEmpty(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function finite(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function color(value: unknown): GroupColor {
  return typeof value === "string" && (groupColorIds as readonly string[]).includes(value)
    ? value as GroupColor
    : "purple";
}

/** Layout nodes are user-editable data, so parse them defensively and bound recursion. */
function layoutNode(value: unknown, depth: number): LayoutNode | null {
  if (depth > 32) return null;
  const raw = object(value);
  if (!raw) return null;
  const id = nonEmpty(raw.id);
  if (!id) return null;
  if (raw.kind === "pane") {
    const sessionId = nonEmpty(raw.sessionId);
    return sessionId ? { kind: "pane", id, sessionId } : null;
  }
  if (raw.kind !== "split" || (raw.axis !== "row" && raw.axis !== "column")) return null;
  const first = layoutNode(raw.first, depth + 1);
  const second = layoutNode(raw.second, depth + 1);
  if (!first || !second) return null;
  return { kind: "split", id, axis: raw.axis, ratio: Math.min(1, Math.max(0, finite(raw.ratio) ?? 0.5)), first, second };
}

function layout(value: unknown): TilingState | null {
  const raw = object(value);
  if (!raw) return null;
  const root = layoutNode(raw.root, 0);
  if (!root) return null;
  // Duplicate node or session ids would draw one terminal twice.
  const ids = nodeIds(root);
  if (new Set(ids).size !== ids.length) return null;
  const open = panes(root);
  if (new Set(open.map((pane) => pane.sessionId)).size !== open.length) return null;
  const focusedPaneId = open.find((pane) => pane.id === raw.focusedPaneId)?.id ?? open[0]?.id ?? null;
  return { root, focusedPaneId, zoomed: raw.zoomed === true && open.length > 1 };
}

/**
 * Reads a stored workspace document. Any corrupt, older or unreadable part is
 * dropped rather than crashing the workspace; the next save rewrites it.
 */
export function parseSnapshot(raw: string): WorkspaceSnapshot | null {
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return null;
  }
  const document = object(value);
  if (!document || document.version !== SNAPSHOT_VERSION) return null;

  const groups: PersistedGroup[] = [];
  const groupIds = new Set<string>();
  if (Array.isArray(document.groups)) {
    for (const entry of document.groups) {
      const group = object(entry);
      const id = group ? nonEmpty(group.id) : null;
      if (!group || !id || groupIds.has(id)) continue;
      groupIds.add(id);
      groups.push({
        id,
        title: typeof group.title === "string" && group.title.trim() ? group.title : id,
        cwd: nonEmpty(group.cwd) ?? "~",
        color: color(group.color),
        collapsed: group.collapsed === true,
      });
    }
  }

  const tabs: PersistedTab[] = [];
  const tabIds = new Set<string>();
  if (Array.isArray(document.tabs)) {
    for (const entry of document.tabs) {
      const tab = object(entry);
      const id = tab ? nonEmpty(tab.id) : null;
      if (!tab || !id || tabIds.has(id)) continue;
      tabIds.add(id);
      const groupId = nonEmpty(tab.groupId);
      const grouped = groupId !== null && groupIds.has(groupId);
      tabs.push({
        id,
        title: typeof tab.title === "string" && tab.title.trim() ? tab.title : "Terminal",
        cwd: nonEmpty(tab.cwd) ?? "~",
        groupId: grouped ? groupId : null,
        pinned: grouped ? false : tab.pinned === true,
        customTitle: tab.customTitle === true,
      });
    }
  }

  const layouts: Record<string, TilingState> = {};
  const storedLayouts = object(document.layouts);
  if (storedLayouts) {
    for (const [key, entry] of Object.entries(storedLayouts)) {
      const state = layout(entry);
      if (state) layouts[key] = state;
    }
  }

  return {
    version: SNAPSHOT_VERSION,
    groups,
    tabs,
    layouts,
    activeTabId: nonEmpty(document.activeTabId),
    workspaceKey: typeof document.workspaceKey === "string" ? document.workspaceKey : "",
    expanded: document.expanded !== false,
    sidebarWidth: finite(document.sidebarWidth),
    fontSize: finite(document.fontSize),
    scrollback: finite(document.scrollback),
    appearance: typeof document.appearance === "string" && appearances.includes(document.appearance)
      ? document.appearance as Appearance
      : "system",
    interfaceFont: nonEmpty(document.interfaceFont),
    monoFont: nonEmpty(document.monoFont),
  };
}

export function serializeSnapshot(snapshot: WorkspaceSnapshot): string {
  return JSON.stringify(snapshot);
}
