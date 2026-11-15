import { keymap, Scroll, signal, Text, token, View, type GestureResponderEvent, type Point, type Signal, type ViewRef } from "nsh";
import { Button } from "nsh/base";
import { Menu } from "nsh/desktop";
import { Icon } from "../shared/icons";
import { Hint, IconButton } from "../shared/controls";
import type { Workspace } from "./workspace";
import { groupColor, groupLine, groupTint } from "../shared/theme";
import { GroupEditor } from "./GroupEditor";
import { Type } from "../shared/type";

type DragSlot = { ref: ViewRef; kind: "tab" | "header" | "ungrouped"; id: string | null };
type SlotRect = { x: number; y: number; width: number; height: number };
type DropTarget = { groupId: string | null; pinned: boolean; beforeId: string | null; headerId: string | null; combineId: string | null };

export interface SidebarActions {
  renameTab(id: string): void;
  newGroup(): void;
  closeGroup(id: string): void;
  search(): void;
  settings(): void;
  toggleSidebar(): void;
}

export function Sidebar(props: { workspace: Workspace; actions: SidebarActions; groupEditor: Signal<string | null>; expanded: () => boolean }) {
  const store = props.workspace;
  const actions = props.actions;
  const members = (groupId: string | null) => store.tabs().filter((t) => t.groupId === groupId && !t.pinned);
  const splitSessions = (groupId: string | null) => {
    const ids = store.layoutSessions(groupId);
    if (ids.length < 2) return [];
    return ids.filter((id) => (store.tab(id)?.groupId ?? null) === groupId);
  };
  const dragging = signal<string | null>(null);
  const drop = signal<DropTarget | null>(null);
  const slots = signal(new Map<string, DragSlot>());
  const rects = signal(new Map<string, SlotRect>());
  const panel = signal<ViewRef | null>(null);
  const dragMoved = signal(false);
  const measureEpoch = signal({ n: 0 });

  const register = (key: string, slot: DragSlot | null) => {
    const map = slots();
    if (slot) map.set(key, slot);
    else map.delete(key);
    scheduleMeasure();
  };

  function scheduleMeasure() {
    const epoch = measureEpoch();
    const generation = ++epoch.n;
    const entries = [...slots().entries()];
    const root = panel();
    void (async () => {
      const next = new Map<string, SlotRect>();
      const frame = await root?.measure();
      if (frame) next.set("panel", { x: frame.pagePosition.x, y: frame.pagePosition.y, width: frame.bounds.size.width, height: frame.bounds.size.height });
      for (const [key, slot] of entries) {
        const measured = await slot.ref.measure();
        if (!measured) continue;
        next.set(key, { x: measured.pagePosition.x, y: measured.pagePosition.y, width: measured.bounds.size.width, height: measured.bounds.size.height });
      }
      if (generation === measureEpoch().n) rects.set(next);
    })();
  }

  function locate(pageX: number, pageY: number): DropTarget | null {
    const sourceId = dragging();
    const source = sourceId ? store.tab(sourceId) : undefined;
    const frame = rects().get("panel");
    if (!source || !frame) return null;
    if (pageX < frame.x - 8 || pageX > frame.x + frame.width + 8 || pageY < frame.y || pageY > frame.y + frame.height) return null;
    let best: { slot: DragSlot; y: number; height: number; distance: number; center: number } | null = null;
    for (const [key, slot] of slots()) {
      const rect = rects().get(key);
      if (!rect) continue;
      const distance = pageY < rect.y ? rect.y - pageY : pageY > rect.y + rect.height ? pageY - (rect.y + rect.height) : 0;
      const center = Math.abs(pageY - (rect.y + rect.height / 2));
      if (!best || distance < best.distance || (distance === best.distance && center < best.center)) {
        best = { slot, y: rect.y, height: rect.height, distance, center };
      }
    }
    if (!best || best.distance > 14) return null;
    return targetFor(source.id, best.slot, pageY, best.y, best.height, best.distance === 0);
  }

  function targetFor(sourceId: string, slot: DragSlot, pageY: number, top: number, height: number, onTab: boolean): DropTarget | null {
    const source = store.tab(sourceId);
    if (!source) return null;
    if (slot.kind === "header" && slot.id) {
      const peers = members(slot.id).filter((t) => t.id !== sourceId);
      return { groupId: slot.id, pinned: false, beforeId: peers[0]?.id ?? null, headerId: slot.id, combineId: null };
    }
    if (slot.kind === "ungrouped") {
      const peers = members(null).filter((t) => t.id !== sourceId);
      return { groupId: null, pinned: false, beforeId: peers[0]?.id ?? null, headerId: null, combineId: null };
    }
    const target = slot.id ? store.tab(slot.id) : undefined;
    if (!target || target.id === sourceId) return null;
    const sameSection = source.groupId === target.groupId && source.pinned === target.pinned;
    const upper = pageY < top + height / 2;
    if (onTab && (!sameSection || !target.groupId)) {
      if (target.groupId) {
        const peers = store.tabs().filter((t) => t.groupId === target.groupId && !t.pinned && t.id !== sourceId);
        const index = peers.findIndex((t) => t.id === target.id);
        return { groupId: target.groupId, pinned: false, beforeId: upper ? target.id : peers[index + 1]?.id ?? null, headerId: null, combineId: target.id };
      }
      return { groupId: null, pinned: false, beforeId: upper ? target.id : null, headerId: null, combineId: target.id };
    }
    if (target.pinned && !source.pinned) return null;
    const peers = store.tabs().filter((t) => t.groupId === target.groupId && t.pinned === target.pinned);
    const index = peers.findIndex((t) => t.id === target.id);
    const beforeId = upper ? target.id : peers[index + 1]?.id ?? null;
    return {
      groupId: target.pinned ? null : target.groupId,
      pinned: target.pinned,
      beforeId: beforeId === sourceId ? sourceId : beforeId,
      headerId: null,
      combineId: null,
    };
  }

  function track(id: string, event: GestureResponderEvent) {
    if (dragging() !== id) return;
    dragMoved.set(true);
    store.trackPaneDrag(event.pageX, event.pageY);
    drop.set(locate(event.pageX, event.pageY));
  }

  function finish(id: string) {
    if (dragging() !== id) return;
    const target = drop();
    dragging.set(null);
    drop.set(null);
    if (store.finishPaneDrag()) { requestAnimationFrame(() => dragMoved.set(false)); return; }
    if (target?.combineId && !target.groupId) store.combineTabs(id, target.combineId, target.beforeId === target.combineId);
    else if (target) {
      store.placeTab(id, { groupId: target.groupId, pinned: target.pinned, beforeId: target.beforeId });
    }
    requestAnimationFrame(() => dragMoved.set(false));
  }

  function releaseDrag() {
    store.cancelPaneDrag();
    dragMoved.set(true);
    dragging.set(null);
    drop.set(null);
    requestAnimationFrame(() => dragMoved.set(false));
  }

  function SplitCard(card: { groupId: string | null }) {
    const ids = () => {
      const all = splitSessions(card.groupId);
      if (!props.expanded()) {
        const active = all.find((id) => id === store.activeId());
        return active ? [active] : [];
      }
      return all;
    };
    const open = () => ids().includes(store.activeId() ?? "");
    return <View testID={card.groupId ? `split-unit-${card.groupId}` : "split-unit"}
      accessibilityLabel={`Split, ${splitSessions(card.groupId).length} terminals`}
      class="mx-1 my-1 rounded-(--radius-panel) border p-0.5"
      style={{ borderColor: token(open() ? "ring" : "border") }}>
      <Each of={ids()}>{(id) => <TabRow id={id()} />}</Each>
    </View>;
  }

  function toggleGroup(id: string) {
    if (dragMoved() || dragging()) return;
    store.patchGroup(id, { collapsed: !store.group(id)?.collapsed });
  }

  const tabMenu = (id: string, at?: Point) => Menu.popup([
    { label: "Rename terminal…", onSelect: () => actions.renameTab(id) },
    { label: store.tab(id)?.pinned ? "Unpin terminal" : "Pin terminal", onSelect: () => store.togglePin(id) },
    { label: store.inSplit(id) ? "Move split to group" : "Move to group", submenu: [
      { label: "No group", checked: !store.tab(id)?.groupId, disabled: store.inSplit(id) && !store.tab(id)?.groupId, onSelect: () => store.moveTab(id, null) },
      ...store.groups().map((g) => ({
        label: store.groupTitle(g.id), checked: store.tab(id)?.groupId === g.id, disabled: store.inSplit(id) && store.tab(id)?.groupId === g.id,
        onSelect: () => store.moveTab(id, g.id),
      })),
    ] },
    { type: "separator" },
    { label: "Move up", onSelect: () => store.reorder(id, -1) },
    { label: "Move down", onSelect: () => store.reorder(id, 1) },
    { type: "separator" },
    { label: "Close terminal", onSelect: () => store.closeTab(id) },
  ], at);

  function TabRow(row: { id: string }) {
    const id = row.id;
    const tab = () => store.tab(id);
    const active = () => store.activeId() === id;
    const combining = () => drop()?.combineId === id && dragging() !== id;
    const lineBefore = () => drop()?.combineId == null && drop()?.headerId == null && drop()?.beforeId === id && dragging() !== id;
    const lineAfter = () => {
      const target = drop();
      const current = tab();
      if (!target || target.combineId || target.headerId || target.beforeId || !current || dragging() === id) return false;
      if (current.groupId !== target.groupId || current.pinned !== target.pinned) return false;
      const peers = store.tabs().filter((item) => item.groupId === current.groupId && item.pinned === current.pinned && item.id !== dragging());
      return peers[peers.length - 1]?.id === id;
    };
    const grouped = () => tab()?.groupId != null && props.expanded();
    const accent = () => {
      const groupId = tab()?.groupId;
      return active() && groupId ? groupColor(store.group(groupId)?.color) : null;
    };
    return <View testID={`tab-row-${id}`}
      ref={(ref) => register(`tab:${id}`, ref ? { ref, kind: "tab", id } : null)}
      class="group relative my-0.5 h-(--height-tab) flex-row items-center rounded-(--radius-tab) hover:bg-tab-hover"
      classList={{ "mx-1": !grouped(), "bg-tab-selected hover:bg-tab-selected": active(), "bg-sidebar-accent": combining() }}
      style={{ opacity: dragging() === id ? 0.45 : 1 }}
      onContextMenu={(e) => { void tabMenu(id, e.pagePosition); }}
      panActivation={{ activeOffsetY: 8, activeOffsetX: 8 }}
      onResponderGrant={() => {
        dragging.set(id);
        store.beginPaneDrag(id);
        drop.set(null);
      }}
      onResponderMove={(event) => track(id, event)}
      onTouchMove={(event) => { if (dragging() === id) track(id, event); }}
      onTouchEnd={() => finish(id)}
      onResponderRelease={() => finish(id)}
      onResponderTerminate={() => releaseDrag()}
      onPanEnd={(event) => { if (event.cancelled) releaseDrag(); else finish(id); }}>
      <If when={combining()}>
        <View testID={`drop-tab-${id}`} accessibilityLabel="Group with terminal" class="absolute inset-0 rounded-(--radius-tab) border border-primary" style={{ pointerEvents: "none" }} />
      </If>
      <If when={lineBefore()}>
        <View testID="drop-line" accessibilityLabel="Insert terminal" class="absolute left-1 right-1 h-0.5 bg-primary" style={{ top: -1, pointerEvents: "none" }} />
      </If>
      <If when={lineAfter()}>
        <View testID="drop-line" accessibilityLabel="Insert terminal" class="absolute left-1 right-1 h-0.5 bg-primary" style={{ bottom: -1, pointerEvents: "none" }} />
      </If>
      <Button testID={`tab-${id}`} role="tab" selected={active()}
        accessibilityLabel={`${tab()?.title ?? "Terminal"}${tab()?.bell ? ", terminal bell" : ""}`} onPress={() => store.select(id)}
        tooltip={<Hint>{tab()?.title ?? "Terminal"}</Hint>}
        class="h-full min-w-0 flex-1 flex-row items-center gap-2 rounded-(--radius-tab) focus-visible:ring-2 ring-sidebar-ring"
        style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 7 : 0 }}>
        <Icon name={tab()?.pinned ? "pin" : "terminal"} size={13} color={accent() ?? token(active() ? "icon-active" : "icon")} />
        <If when={props.expanded()}>
          <Text class="min-w-0 flex-1 text-(length:--text-body)"
            style={{ numberOfLines: 1, color: token(active() ? "tab-selected-foreground" : "sidebar-foreground") }}>{tab()?.title}</Text>
        </If>
      </Button>
      <If when={!!store.paneForSession(id) && !active()}>
        <View class="absolute top-0 bottom-0 w-1.5 items-center justify-center opacity-100 group-hover:opacity-0 group-focus-within:opacity-0"
          style={{ right: 10, pointerEvents: "none" }}>
          <View accessibilityLabel={store.workspaceKey() ? "Shown in this group's layout" : "Shown in the layout"}
            tooltip={<Hint>{store.workspaceKey() ? "Shown in this group's layout" : "Shown in the layout"}</Hint>}
            class="h-1.5 w-1.5 rounded-full bg-icon" />
        </View>
      </If>
      <If when={tab()?.bell}>
        <View accessibilityLabel="Terminal bell" class="absolute right-1 top-1 h-1.5 w-1.5 rounded-full bg-primary" style={{ pointerEvents: "none" }} />
      </If>
      <If when={props.expanded()}>
        <View class="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100" style={{ opacity: active() ? 1 : undefined }}>
          <IconButton icon="close" label={`Close ${tab()?.title}`} testID={`close-${id}`} onPress={() => store.closeTab(id)} />
        </View>
      </If>
    </View>;
  }

  function GroupRow(row: { id: string }) {
    const id = row.id;
    const shownSplit = () => {
      const all = splitSessions(id);
      if (all.length < 2) return [];
      if (store.group(id)?.collapsed && props.expanded()) return [];
      if (!props.expanded()) return all.includes(store.activeId() ?? "") ? [store.activeId()!] : [];
      return all;
    };
    const visibleMembers = () => members(id).filter((tab) => !splitSessions(id).includes(tab.id) && (!store.group(id)?.collapsed || (!props.expanded() && tab.id === store.activeId())));
    const hiddenCount = () => members(id).filter((tab) => !visibleMembers().some((item) => item.id === tab.id) && !shownSplit().includes(tab.id)).length;
    const anchor = signal<ViewRef | null>(null);
    let trigger: ViewRef | undefined;
    const close = (restoreFocus = false) => {
      props.groupEditor.set(null);
      if (restoreFocus) trigger?.focus();
    };
    return <View class="relative mb-2"
        classList={{ "mx-1 py-0.5": props.expanded() }}>
        <If when={!props.expanded()}>
          <View class="absolute w-px rounded-sm" style={{ left: 2, top: 2, bottom: 2, backgroundColor: groupLine(store.group(id)?.color) }} />
        </If>
        <View ref={(ref) => { anchor.set(ref); register(`header:${id}`, ref ? { ref, kind: "header", id } : null); }}
          testID={store.workspaceKey() === id ? `group-workspace-${id}` : undefined}
          class="group relative flex-row items-center rounded-(--radius-control)"
          classList={{ "mx-1": !props.expanded(), "bg-sidebar-accent": drop()?.headerId === id || store.workspaceKey() === id }}
          style={{ marginVertical: props.expanded() ? 0 : 2 }}
          onContextMenu={() => props.groupEditor.set(id)}>
          <If when={drop()?.headerId === id}>
            <View testID={`drop-group-${id}`} class="absolute inset-0 rounded-(--radius-control) border border-primary" style={{ pointerEvents: "none" }} />
          </If>
          <Button testID={`group-${id}`} ref={(ref) => { trigger = ref; }}
            accessibilityLabel={`${store.group(id)?.collapsed ? "Expand" : "Collapse"} ${store.groupTitle(id)}${store.workspaceKey() === id ? ", current layout" : ""}`}
            onPress={() => { close(false); toggleGroup(id); }}
            class="h-(--height-tab) min-w-0 flex-1 flex-row items-center gap-1.5 rounded-(--radius-control) focus-visible:ring-2 ring-sidebar-ring"
            style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 4 : 0 }}
            tooltip={<Hint>{`${store.groupTitle(id)} · ${store.group(id)?.cwd ?? ""} · ${members(id).length} terminals`}</Hint>}>
            <If when={props.expanded()}>
              <Icon name={store.group(id)?.collapsed ? "chevron" : "down"} size={11} color={token("subtle-foreground")} />
            </If>
            <View accessibilityLabel={`${store.group(id)?.color ?? "blue"} group marker`} style={{
              width: token("size-group-marker"),
              height: token("size-group-marker"),
              borderRadius: token("size-group-marker") / 2,
              borderBottomLeftRadius: token("radius-group-corner"),
              backgroundColor: groupColor(store.group(id)?.color),
            }} />
            <If when={props.expanded()}>
              <Text class="min-w-0 flex-1 text-(length:--text-body) font-medium"
                style={{ color: token("sidebar-foreground"), numberOfLines: 1 }}>{store.groupTitle(id)}</Text>
            </If>
          </Button>
          <If when={props.expanded()}>
            <View class="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
              style={{ opacity: props.groupEditor() === id ? 1 : undefined }}>
              <IconButton icon="more" label={`Options for ${store.groupTitle(id)}`} testID={`group-menu-${id}`} onPress={() => props.groupEditor.set(props.groupEditor() === id ? null : id)} />
            </View>
          </If>
        </View>
        <If when={shownSplit().length > 0 || visibleMembers().length > 0}>
          <View style={{ marginLeft: props.expanded() ? 10 : 0, paddingLeft: props.expanded() ? token("gap-group-guide") : 0, borderLeftWidth: props.expanded() ? 1 : 0, borderColor: groupLine(store.group(id)?.color) }}>
            <If when={shownSplit().length > 0}><SplitCard groupId={id} /></If>
            <Each of={visibleMembers().map((t) => t.id)}>{(tabId) => <TabRow id={tabId()} />}</Each>
          </View>
        </If>
        <If when={!props.expanded() && hiddenCount() > 0}>
          <Button testID={`group-hidden-${id}`} accessibilityLabel={`Show ${hiddenCount()} hidden terminals in ${store.groupTitle(id)}`}
            onPress={() => store.patchGroup(id, { collapsed: false })}
            class="mx-1 my-0.5 h-(--height-tab) items-center justify-center rounded-(--radius-tab) focus-visible:ring-2 ring-sidebar-ring">
            <View class="h-(--height-tab) w-(--height-tab) items-center justify-center rounded-full"
              style={{ backgroundColor: groupTint(store.group(id)?.color) }}>
              <Text class="text-(length:--text-label)" style={{ color: groupColor(store.group(id)?.color) }}>+{hiddenCount()}</Text>
            </View>
          </Button>
        </If>
      <If when={props.groupEditor() === id && anchor() !== null}>
          <GroupEditor id={id} anchor={anchor()!} workspace={store} close={close}
            deleteGroup={() => actions.closeGroup(id)} />
        </If>
      </View>;
  }

  const appendLine = (groupId: string | null, pinned: boolean) => {
    const target = drop();
    if (!target || target.headerId || target.beforeId || target.groupId !== groupId || target.pinned !== pinned || !dragging()) return false;
    const peers = store.tabs().filter((t) => t.groupId === groupId && t.pinned === pinned && t.id !== dragging());
    return peers.length === 0;
  };

  return <View ref={(ref) => panel.set(ref)} class="h-full">
    <View testID="sidebar-heading" class="h-(--height-toolbar) shrink-0 flex-row items-center px-3"
      style={{ justifyContent: props.expanded() ? "flex-start" : "center" }}>
      <If when={props.expanded()}><Type role="label">Sessions</Type></If>
      <If when={!props.expanded()}><Icon name="terminal" size={14} /></If>
    </View>

    <Scroll class="min-h-0 flex-1 pt-1.5" scrollbar={false}>
      <If when={splitSessions(null).length >= 2 && props.expanded()}><SplitCard groupId={null} /></If>
      <If when={splitSessions(null).length >= 2 && !props.expanded() && splitSessions(null).includes(store.activeId() ?? "")}>
        <SplitCard groupId={null} />
      </If>
      <If when={store.tabs().some((t) => t.pinned && !splitSessions(null).includes(t.id))}>
        <Each of={store.tabs().filter((t) => t.pinned && !splitSessions(null).includes(t.id)).map((t) => t.id)}>{(id) => <TabRow id={id()} />}</Each>
        <View ref={(ref) => register("ungrouped", ref ? { ref, kind: "ungrouped", id: null } : null)}
          testID="ungrouped-drop" class="mx-2 my-2 h-px bg-sidebar-border" />
      </If>
      <If when={!store.tabs().some((t) => t.pinned && !splitSessions(null).includes(t.id))}>
        <View ref={(ref) => register("ungrouped", ref ? { ref, kind: "ungrouped", id: null } : null)}
          testID="ungrouped-drop" class="mx-2 h-2" />
      </If>
      <Each of={members(null).filter((t) => !splitSessions(null).includes(t.id)).map((t) => t.id)}>{(id) => <TabRow id={id()} />}</Each>
      <If when={appendLine(null, false)}>
        <View testID="drop-line" accessibilityLabel="Insert terminal" class="mx-3 h-0.5 bg-primary" />
      </If>

      <Each of={store.groups().map((g) => g.id)}>{(id) => <GroupRow id={id()} />}</Each>

      <Button testID="new-terminal" accessibilityLabel="New terminal" onPress={() => store.addTab()}
        class="mx-1 mb-2 h-(--height-tab) flex-row items-center gap-2 rounded-(--radius-tab) px-2 hover:bg-sidebar-accent focus-visible:ring-2 ring-sidebar-ring"
        style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 8 : 0 }}>
        <Icon name="plus" size={13} />
        <If when={props.expanded()}>
          <Text class="flex-1 text-(length:--text-label) text-muted-foreground">New terminal</Text>
          <Text class="text-(length:--text-caption) text-subtle-foreground">{keymap.label("mod+t")}</Text>
        </If>
      </Button>
    </Scroll>
  </View>;
}
