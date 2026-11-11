import { batch, computed, signal, token } from "nsh";
import { groupPalette, type GroupColor } from "./theme";

export interface Tab {
  id: string;
  title: string;
  groupId: string | null;
  pinned: boolean;
  cwd: string;
}

export interface TabGroup {
  id: string;
  title: string;
  color: GroupColor;
  collapsed: boolean;
}

export function createWorkspace() {
  // IDs, not tab objects or their positions, own the terminal surfaces.
  const tabs = signal<Tab[]>([
    { id: "home", title: "Terminal", groupId: null, pinned: true, cwd: "~" },
    { id: "prototype", title: "UI prototype", groupId: "termlevel", pinned: false, cwd: "~/Projects/termlevel" },
    { id: "preview", title: "Build & preview", groupId: "termlevel", pinned: false, cwd: "~/Projects/termlevel" },
    { id: "colors", title: "Color test", groupId: "playground", pinned: false, cwd: "~/Playground" },
  ]);
  const groups = signal<TabGroup[]>([
    { id: "termlevel", title: "Termlevel", color: "purple", collapsed: false },
    { id: "playground", title: "Playground", color: "blue", collapsed: false },
  ]);
  const activeId = signal<string | null>("prototype");
  const expanded = signal(true);
  const sidebarWidth = signal(token("width-sidebar"));
  const fontSize = signal(token("text-terminal"));
  const activeTab = computed(() => tabs().find((t) => t.id === activeId()));
  let sequence = 0;

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
      const groupId = tab(id)?.groupId;
      if (groupId) patchGroup(groupId, { collapsed: false });
    });
  }

  function addTab(groupId: string | null = activeTab()?.groupId ?? null) {
    const id = `tab-${++sequence}`;
    const validGroup = groupId && group(groupId) ? groupId : null;
    batch(() => {
      tabs.update((all) => [...all, {
        id, title: `Terminal ${sequence}`, groupId: validGroup, pinned: false,
        cwd: activeTab()?.cwd ?? "~",
      }]);
      select(id);
    });
    return id;
  }

  function closeTab(id: string) {
    const all = tabs();
    const index = all.findIndex((t) => t.id === id);
    if (index < 0) return;
    batch(() => {
      tabs.set(all.filter((t) => t.id !== id));
      if (activeId() === id) {
        activeId.set(all[index + 1]?.id ?? all[index - 1]?.id ?? null);
        if (activeId()) select(activeId()!);
      }
    });
  }

  function addGroup(title: string) {
    const id = `group-${++sequence}`;
    batch(() => {
      groups.update((all) => [...all, {
        id, title, color: groupPalette[0].id, collapsed: false,
      }]);
      expanded.set(true);
      addTab(id);
    });
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

  function moveTab(id: string, destination: string | null) {
    if (destination && !group(destination)) return;
    batch(() => {
      patchTab(id, { groupId: destination, pinned: false });
      if (destination) patchGroup(destination, { collapsed: false });
    });
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
    ungroup, closeGroup, moveTab, togglePin, step, reorder,
  };
}

export type Workspace = ReturnType<typeof createWorkspace>;
