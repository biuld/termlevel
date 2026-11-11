import { keymap, Scroll, signal, Text, token, View, type Point, type Signal, type ViewRef } from "nsh";
import { Button } from "nsh/base";
import { Menu, titleBar } from "nsh/desktop";
import { Icon } from "./icons";
import { IconButton } from "./controls";
import type { Workspace } from "./workspace";
import { groupColor, groupLine, groupTint } from "./theme";
import { GroupEditor } from "./GroupEditor";

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

  const tabMenu = (id: string, at?: Point) => Menu.popup([
    { label: "Rename terminal…", onSelect: () => actions.renameTab(id) },
    { label: store.tab(id)?.pinned ? "Unpin terminal" : "Pin terminal", onSelect: () => store.togglePin(id) },
    { label: "Move to group", submenu: [
      { label: "No group", checked: !store.tab(id)?.groupId, onSelect: () => store.moveTab(id, null) },
      ...store.groups().map((g) => ({
        label: store.groupTitle(g.id), checked: store.tab(id)?.groupId === g.id, onSelect: () => store.moveTab(id, g.id),
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
    return <View testID={`tab-row-${id}`}
      class="group relative mx-1 my-0.5 h-(--height-tab) flex-row items-center rounded-(--radius-tab) hover:bg-tab-hover"
      classList={{ "bg-tab-selected hover:bg-tab-selected": active() }}
      onContextMenu={(e) => { void tabMenu(id, e.pagePosition); }}>
      <Button testID={`tab-${id}`} role="tab" selected={active()}
        accessibilityLabel={tab()?.title ?? "Terminal"} onPress={() => store.select(id)}
        tooltip={<Text class="rounded-(--radius-control) border border-border bg-popover px-2 py-1 text-(length:--text-caption) text-popover-foreground">{tab()?.title}</Text>}
        class="h-full min-w-0 flex-1 flex-row items-center gap-2 rounded-(--radius-tab) focus-visible:ring-2 ring-sidebar-ring"
        style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 7 : 0 }}>
        <Icon name={tab()?.pinned ? "pin" : "terminal"} size={13} color={token(active() ? "icon-active" : "icon")} />
        <If when={props.expanded()}>
          <Text class="min-w-0 flex-1 text-(length:--text-body)"
            style={{ numberOfLines: 1, color: token(active() ? "tab-selected-foreground" : "sidebar-foreground") }}>{tab()?.title}</Text>
        </If>
      </Button>
      <If when={props.expanded()}>
        <View class="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100" style={{ opacity: active() ? 1 : undefined }}>
          <IconButton icon="close" label={`Close ${tab()?.title}`} testID={`close-${id}`} onPress={() => store.closeTab(id)} />
        </View>
      </If>
    </View>;
  }

  function GroupRow(row: { id: string }) {
    const id = row.id;
    const visibleMembers = () => members(id).filter((tab) => !store.group(id)?.collapsed || (!props.expanded() && tab.id === store.activeId()));
    const hiddenCount = () => members(id).length - visibleMembers().length;
    const anchor = signal<ViewRef | null>(null);
    let trigger: ViewRef | undefined;
    const close = (restoreFocus = false) => {
      props.groupEditor.set(null);
      if (restoreFocus) trigger?.focus();
    };
    return <View class="relative mb-2">
        <If when={!props.expanded()}>
          <View class="absolute w-0.5 rounded-sm" style={{ left: 2, top: 2, bottom: 2, backgroundColor: groupColor(store.group(id)?.color) }} />
        </If>
        <View ref={(ref) => anchor.set(ref)} class="group mx-1 flex-row items-center rounded-(--radius-control)"
          style={{ marginVertical: props.expanded() ? 0 : 2 }}
          onContextMenu={() => props.groupEditor.set(id)}>
          <Button testID={`group-${id}`} ref={(ref) => { trigger = ref; }}
            accessibilityLabel={`${store.group(id)?.collapsed ? "Expand" : "Collapse"} ${store.groupTitle(id)}`}
            onPress={() => { close(false); store.patchGroup(id, { collapsed: !store.group(id)?.collapsed }); }}
            class="h-(--height-tab) min-w-0 flex-1 flex-row items-center gap-1.5 rounded-(--radius-control) hover:bg-sidebar-accent focus-visible:ring-2 ring-sidebar-ring"
            style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 4 : 0 }}
            tooltip={<Text class="rounded-(--radius-control) border border-border bg-popover px-2 py-1 text-(length:--text-caption) text-popover-foreground">{store.groupTitle(id)} · {members(id).length} terminals</Text>}>
            <If when={props.expanded()}>
              <Icon name={store.group(id)?.collapsed ? "chevron" : "down"} size={11} color={token("subtle-foreground")} />
            </If>
            <View style={{
              width: token(props.expanded() ? "size-group-marker" : "height-tab"),
              height: token(props.expanded() ? "size-group-marker" : "height-tab"),
              borderRadius: token(props.expanded() ? "size-group-marker" : "height-tab") / 2,
              borderBottomLeftRadius: token("radius-group-corner"),
              backgroundColor: groupColor(store.group(id)?.color),
            }} />
            <If when={props.expanded()}>
              <Text class="min-w-0 flex-1 text-(length:--text-body) font-medium"
                style={{ color: groupColor(store.group(id)?.color), numberOfLines: 1 }}>{store.groupTitle(id)}</Text>
            </If>
          </Button>
          <If when={props.expanded()}>
            <View class="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
              style={{ opacity: props.groupEditor() === id ? 1 : undefined }}>
              <IconButton icon="more" label={`Options for ${store.groupTitle(id)}`} testID={`group-menu-${id}`} onPress={() => props.groupEditor.set(props.groupEditor() === id ? null : id)} />
            </View>
          </If>
        </View>
        <If when={visibleMembers().length > 0}>
          <View style={{ marginLeft: props.expanded() ? 10 : 0, borderLeftWidth: props.expanded() ? 1 : 0, borderColor: groupLine(store.group(id)?.color) }}>
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

  return <View class="h-full bg-sidebar">
    <If when={store.expanded()}>
      <View testID="sidebar-titlebar" desktop={{ windowDrag: true }}
        class="h-(--height-titlebar) shrink-0 flex-row items-center justify-end"
        style={{ paddingLeft: titleBar().left + 8, paddingRight: 6 }}>
        <View desktop={{ windowDrag: false }}>
          <IconButton icon="sidebar" label="Collapse sidebar" testID="toggle-sidebar" onPress={actions.toggleSidebar} />
        </View>
      </View>
    </If>

    <Scroll class="min-h-0 flex-1 pt-1.5" scrollbar={false}>
      <If when={store.tabs().some((t) => t.pinned)}>
        <Each of={store.tabs().filter((t) => t.pinned).map((t) => t.id)}>{(id) => <TabRow id={id()} />}</Each>
        <View class="mx-2 my-2 h-px bg-sidebar-border" />
      </If>
      <Each of={members(null).map((t) => t.id)}>{(id) => <TabRow id={id()} />}</Each>

      <Each of={store.groups().map((g) => g.id)}>{(id) => <GroupRow id={id()} />}</Each>

      <Button testID="new-terminal" accessibilityLabel="New terminal" onPress={() => store.addTab()}
        class="mx-1 mb-2 h-(--height-tab) flex-row items-center gap-2 rounded-(--radius-tab) px-2 hover:bg-sidebar-accent focus-visible:ring-2 ring-sidebar-ring"
        style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 8 : 0 }}>
        <Icon name="plus" size={13} />
        <If when={props.expanded()}>
          <Text class="flex-1 text-(length:--text-label) text-muted-foreground">New terminal</Text>
          <Text class="text-(length:--text-micro) text-subtle-foreground">{keymap.label("mod+t")}</Text>
        </If>
      </Button>
    </Scroll>

    <View class="gap-0.5 border-t border-sidebar-border p-1" style={{ flexDirection: props.expanded() ? "row" : "column", alignItems: "center" }}>
      <If when={props.expanded()}>
        <Text class="min-w-0 flex-1 pl-1 text-(length:--text-caption) text-muted-foreground">{store.tabs().length} {store.tabs().length === 1 ? "terminal" : "terminals"}</Text>
      </If>
      <IconButton icon="folder" label="New group" testID="new-group" onPress={actions.newGroup} />
      <IconButton icon="search" label="Find a terminal" testID="find-terminal" onPress={actions.search} />
      <IconButton icon="settings" label="Settings" testID="settings" onPress={actions.settings} />
    </View>
  </View>;
}
