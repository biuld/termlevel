import { batch, computed, signal, token } from "nsh";
import { groupPalette, type GroupColor } from "./theme";

export interface Tab {
  id: string;
  title: string;
  groupId: string | null;
  pinned: boolean;
  cwd: string;
  state?: string;
  exitCode?: number | null;
  error?: string | null;
  customTitle?: boolean;
  bell?: boolean;
}

export interface SessionTransport {
  cwd: string;
  list(): Array<{ id: string; cwd: string; title: string; state: string; exitCode?: number | null; error?: string | null }>;
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

export function createWorkspace(transport?: SessionTransport) {
  const restored = transport?.list() ?? [];
  const initial = transport ? (restored.length ? restored : [transport.open(transport.cwd)]) : null;
  // IDs, not tab objects or their positions, own the terminal surfaces.
  const tabs = signal<Tab[]>(initial ? initial.map((session, index) => ({
    ...session, title: session.title || `Terminal ${index + 1}`, groupId: null, pinned: false,
  })) : [
    { id: "home", title: "Terminal", groupId: null, pinned: true, cwd: "~" },
    { id: "prototype", title: "UI prototype", groupId: "termlevel", pinned: false, cwd: "~/Projects/termlevel" },
    { id: "preview", title: "Build & preview", groupId: "termlevel", pinned: false, cwd: "~/Projects/termlevel" },
    { id: "colors", title: "Color test", groupId: "playground", pinned: false, cwd: "~/Playground" },
  ]);
  const groups = signal<TabGroup[]>(transport ? [] : [
    { id: "termlevel", title: "termlevel", cwd: "~/Projects/termlevel", color: "purple", collapsed: false },
    { id: "playground", title: "Playground", cwd: "~/Playground", color: "blue", collapsed: false },
  ]);
  const activeId = signal<string | null>(initial ? initial[0]?.id ?? null : "prototype");
  const expanded = signal(true);
  const sidebarWidth = signal(token("width-sidebar"));
  const fontSize = signal(token("text-terminal"));
  const activeTab = computed(() => tabs().find((t) => t.id === activeId()));
  let sequence = initial?.length ?? 0;

  const tab = (id: string) => tabs().find((t) => t.id === id);
  const group = (id: string) => groups().find((g) => g.id === id);
  const groupTitle = (id: string | null) => id ? group(id)?.title.trim() || "未命名分组" : "Ungrouped";
  const patchTab = (id: string, patch: Partial<Omit<Tab, "id">>) =>
    tabs.update((all) => all.map((t) => t.id === id ? { ...t, ...patch } : t));
  const patchGroup = (id: string, patch: Partial<Omit<TabGroup, "id">>) =>
    groups.update((all) => all.map((g) => g.id === id ? { ...g, ...patch } : g));

  function select(id: string) {
    if (!tab(id)) return;
    batch(() => {
      activeId.set(id);
      if (tab(id)?.bell) patchTab(id, { bell: false });
      const groupId = tab(id)?.groupId;
      if (groupId) patchGroup(groupId, { collapsed: false });
    });
  }

  function addTab(groupId: string | null = null) {
    sequence++;
    const destination = groupId ? group(groupId) : undefined;
    const cwd = destination?.cwd || activeTab()?.cwd || transport?.cwd || "~";
    const session = transport?.open(cwd);
    const id = session?.id ?? `tab-${sequence}`;
    const validGroup = destination ? groupId : null;
    batch(() => {
      tabs.update((all) => [...all, {
        ...session, id, title: session?.title || `Terminal ${sequence}`, groupId: validGroup, pinned: false,
        cwd: session?.cwd ?? cwd,
      }]);
      select(id);
    });
    return id;
  }

  function closeTab(id: string) {
    const all = tabs();
    const index = all.findIndex((t) => t.id === id);
    if (index < 0) return;
    transport?.close(id);
    batch(() => {
      tabs.set(all.filter((t) => t.id !== id));
      if (activeId() === id) {
        activeId.set(all[index + 1]?.id ?? all[index - 1]?.id ?? null);
        if (activeId()) select(activeId()!);
      }
    });
  }

  /** A palette color not already on a group, or any color once all nine are used. */
  function nextGroupColor(): GroupColor {
    const used = new Set(groups().map((item) => item.color));
    const open = groupPalette.filter((item) => !used.has(item.id));
    const pool = open.length > 0 ? open : [...groupPalette];
    return pool[Math.floor(Math.random() * pool.length)].id;
  }

  function addGroup() {
    const cwd = activeTab()?.cwd || transport?.cwd || "~";
    const id = `group-${++sequence}`;
    const color = nextGroupColor();
    batch(() => {
      groups.update((all) => [...all, {
        id, title: pathName(cwd), cwd, color, collapsed: false,
      }]);
      expanded.set(true);
    });
    addTab(id);
    return id;
  }

  function ungroup(id: string) {
    batch(() => {
      tabs.update((all) => all.map((t) => t.groupId === id ? { ...t, groupId: null } : t));
      groups.update((all) => all.filter((g) => g.id !== id));
    });
  }

  function closeGroup(id: string) {
    batch(() => {
      for (const t of tabs().filter((t) => t.groupId === id)) closeTab(t.id);
      groups.update((all) => all.filter((g) => g.id !== id));
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
    batch(() => {
      tabs.set(next);
      if (groupId) patchGroup(groupId, { collapsed: false });
    });
  }

  function moveTab(id: string, destination: string | null) {
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
    batch(() => {
      groups.update((all) => [...all, { id: groupId, title: pathName(cwd), cwd, color, collapsed: false }]);
      tabs.set(next);
      expanded.set(true);
    });
    return groupId;
  }

  function togglePin(id: string) {
    const current = tab(id);
    if (current) patchTab(id, { pinned: !current.pinned, groupId: null });
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

  return {
    tabs, groups, activeId, activeTab, expanded, sidebarWidth, fontSize,
    tab, group, groupTitle, patchTab, patchGroup, select, addTab, closeTab, addGroup,
    ungroup, closeGroup, moveTab, placeTab, combineTabs, togglePin, step, reorder,
  };
}

export type Workspace = ReturnType<typeof createWorkspace>;
