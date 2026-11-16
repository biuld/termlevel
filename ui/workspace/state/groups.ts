import { batch, type Signal } from "nsh";
import { groupPalette, type GroupColor } from "../../shared/theme";
import { panes, type Pane, type TilingState } from "../tiling";
import type { GitView } from "../workspace-snapshot";
import { groupName } from "./initial";
import type { Tab, TabGroup } from "./types";

type State<T> = Signal<T>;
type Context = {
  groups: State<TabGroup[]>; tabs: State<Tab[]>; expanded: State<boolean>;
  workspaceKey: State<string>; layouts: State<Record<string, TilingState>>;
  gitViews: State<Record<string, { open: GitView[]; selected: GitView | null }>>;
  activeId: () => string | null; nextSequence: () => number;
  tab: (id: string) => Tab | undefined; group: (id: string) => TabGroup | undefined;
  addTab: (id?: string | null, focus?: boolean) => unknown; closeTabs: (ids: string[]) => void;
  layoutOf: (key: string) => TilingState; layoutKey: (id: string | null | undefined) => string;
  layoutSessions: (id: string | null) => string[]; placeSplit: (from: string | null, to: string | null) => void;
  select: (id: string, acknowledge?: boolean) => void; forgetSession: (id: string) => void;
  patchGroup: (id: string, patch: Partial<Omit<TabGroup, "id">>) => void;
  nextPane: (id: string) => Pane; setLayout: (key: string, state: TilingState) => void;
};

export function groupOperations(context: Context) {
  const { groups, tabs, expanded, workspaceKey, layouts, gitViews, activeId, nextSequence, tab, group, addTab, closeTabs, layoutOf, layoutKey, layoutSessions, placeSplit, select, forgetSession, patchGroup, nextPane, setLayout } = context;
  /** A palette color not already on a group, or any color once all nine are used. */
  function nextGroupColor(): GroupColor {
    const used = new Set(groups().map((item) => item.color));
    const open = groupPalette.filter((item) => !used.has(item.id));
    const pool = open.length > 0 ? open : [...groupPalette];
    return pool[Math.floor(Math.random() * pool.length)].id;
  }

  function addGroup(cwd: string) {
    const directory = cwd.trim() || "~";
    const id = `group-${nextSequence()}`;
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
      gitViews.update(all => { const copy = { ...all }; delete copy[id]; return copy; });
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
      } else if (viewing) {
        workspaceKey.set("");
      }
    });
  }

  function closeGroup(id: string) {
    batch(() => {
      gitViews.update(all => { const copy = { ...all }; delete copy[id]; return copy; });
      closeTabs(tabs().filter(t => t.groupId === id).map(t => t.id));
      groups.update((all) => all.filter((g) => g.id !== id));
      if (workspaceKey() === id) {
        const fallback = tabs()[0];
        if (fallback) select(fallback.id);
        else workspaceKey.set("");
      }
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
   * A group destination expands the group. */
  function placeTab(id: string, destination: { groupId: string | null; pinned: boolean; beforeId: string | null }) {
    const current = tab(id);
    if (!current) return;
    if (destination.beforeId === id && destination.groupId === current.groupId && destination.pinned === current.pinned) return;
    if (destination.groupId && !group(destination.groupId)) return;
    const pinned = destination.pinned;
    const groupId = destination.groupId;
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
    if (current) placeTab(id, { groupId: destination, pinned: current.pinned, beforeId: null });
  }

  /** Drops `sourceId` on a tab that is not already in a group.
   * Both land in a new group, `sourceId` before the target when `before` is set.
   * Session ids and pins stay the same. */
  function combineTabs(sourceId: string, targetId: string, before: boolean) {
    const source = tab(sourceId);
    const target = tab(targetId);
    if (!source || !target || sourceId === targetId || target.groupId) return null;
    const groupId = `group-${nextSequence()}`;
    const ordered = (before ? [source, target] : [target, source]).map((item) => ({ ...item, groupId }));
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
    placeTab(id, { groupId: current.groupId, pinned: !current.pinned, beforeId: null });
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

  return { addGroup, ungroup, closeGroup, placeTab, inSplit, moveTab, combineTabs, togglePin, step, reorder };
}
