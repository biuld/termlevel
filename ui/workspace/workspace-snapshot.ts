import { parseRepositoryPreferences, type RepositoryPreferences } from "./repository-preferences";
import { groupColorIds, type GroupColor } from "../shared/theme";
import type { WorkingIndicatorKind } from "../shared/WorkingIndicator";
import { nodeIds, panes, type LayoutNode, type TilingState } from "./tiling";

/**
 * The persisted workspace structure. Sessions are not part of it: a tab keeps
 * its last live directory and title, and the next launch starts a fresh shell
 * in that directory.
 * Additive preferences have defaults; incompatible schema versions are discarded.
 */
export const SNAPSHOT_VERSION = 1;

export type TerminalIdentity = "termlevel" | "ghostty";

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
  terminalIdentity: TerminalIdentity;
  appearance: Appearance;
  interfaceFont: string;
  monoFont: string;
  workingIndicator: WorkingIndicatorKind;
}

export interface WorkspaceSnapshot {
  version: number;
  groups: PersistedGroup[];
  tabs: PersistedTab[];
  layouts: Record<string, TilingState>;
  activeTabId: string | null;
  workspaceKey: string;
  /** Groups whose overview is their remembered view. */
  dashboards: string[];
  /** Open Git views and last selected view for each group. */
  gitViews?: Record<string, { open: GitView[]; selected: GitView | null }>;
  /** File view for each group. Absent means the view was never opened. */
  files?: Record<string, FilesView>;
  gitRepositories?: RepositoryPreferences;
  expanded: boolean;
  sidebarWidth: number | null;
  fontSize: number | null;
  scrollback: number | null;
  terminalIdentity: TerminalIdentity;
  appearance: Appearance;
  /** Null when the stored family is absent or not a known choice. */
  interfaceFont: string | null;
  monoFont: string | null;
  workingIndicator: WorkingIndicatorKind;
}

export type GitView = "changes" | "history";

export interface FilesView {
  open: boolean;
  showing: boolean;
  showHidden: boolean;
  expanded: string[];
  selected: string | null;
}

const appearances: readonly string[] = ["system", "light", "dark"];
const workingIndicators: readonly string[] = ["bars", "vinyl", "wave", "beat"];

function object(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function filePath(value: unknown): string | null {
  if (typeof value !== "string" || !value || value.includes("\0") || value.startsWith("/") || value.split("/").includes("..")) {
    return null;
  }
  return value;
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
        pinned: tab.pinned === true,
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

  const dashboards: string[] = [];
  if (Array.isArray(document.dashboards)) {
    for (const entry of document.dashboards) {
      const id = nonEmpty(entry);
      if (id && groupIds.has(id) && !dashboards.includes(id)) dashboards.push(id);
    }
  }

  const gitViews: NonNullable<WorkspaceSnapshot["gitViews"]> = {};
  const storedGitViews = object(document.gitViews);
  if (storedGitViews) for (const [id, entry] of Object.entries(storedGitViews)) {
    const state = object(entry);
    if (!groupIds.has(id) || !state) continue;
    const open: GitView[] = [];
    if (Array.isArray(state.open)) for (const kind of state.open) {
      if ((kind === "changes" || kind === "history") && !open.includes(kind)) open.push(kind);
    }
    const selected = state.selected === "changes" || state.selected === "history" ? state.selected : null;
    gitViews[id] = { open, selected: selected && open.includes(selected) ? selected : null };
  }
  // An old overview showed Changes by default; preserve that open view on upgrade.
  if (!storedGitViews) for (const id of dashboards) gitViews[id] = { open: ["changes"], selected: "changes" };

  const files: NonNullable<WorkspaceSnapshot["files"]> = {};
  const storedFiles = object(document.files);
  if (storedFiles) for (const [id, entry] of Object.entries(storedFiles)) {
    const state = object(entry);
    if (!groupIds.has(id) || !state) continue;
    const expanded: string[] = [];
    if (Array.isArray(state.expanded)) for (const path of state.expanded) {
      const relative = filePath(path);
      if (expanded.length === 64) break;
      if (relative && !expanded.includes(relative)) expanded.push(relative);
    }
    const selected = filePath(state.selected);
    files[id] = {
      open: state.open === true,
      showing: state.showing === true && state.open === true,
      showHidden: state.showHidden === true,
      expanded,
      selected,
    };
  }

  return {
    version: SNAPSHOT_VERSION,
    groups,
    tabs,
    layouts,
    activeTabId: nonEmpty(document.activeTabId),
    workspaceKey: typeof document.workspaceKey === "string" ? document.workspaceKey : "",
    dashboards,
    gitViews,
    files,
    gitRepositories: parseRepositoryPreferences(document.gitRepositories, groupIds),
    expanded: document.expanded !== false,
    sidebarWidth: finite(document.sidebarWidth),
    fontSize: finite(document.fontSize),
    scrollback: finite(document.scrollback),
    terminalIdentity: document.terminalIdentity === "ghostty" ? "ghostty" : "termlevel",
    appearance: typeof document.appearance === "string" && appearances.includes(document.appearance)
      ? document.appearance as Appearance
      : "system",
    interfaceFont: nonEmpty(document.interfaceFont),
    monoFont: nonEmpty(document.monoFont),
    workingIndicator: typeof document.workingIndicator === "string" && workingIndicators.includes(document.workingIndicator)
      ? document.workingIndicator as WorkingIndicatorKind
      : "bars",
  };
}

export function serializeSnapshot(snapshot: WorkspaceSnapshot): string {
  return JSON.stringify(snapshot);
}
