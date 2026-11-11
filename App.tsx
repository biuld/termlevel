import { effect, keymap, mount, platform, setColorScheme, signal, Text, token, View } from "nsh";
import { Button, Dialog, Input, ResizablePanel, ResizablePanelGroup } from "nsh/base";
import { currentWindow, Menu, titleBar } from "nsh/desktop";
import { Runtime } from "termlevel/native";
import { TerminalSurface } from "./terminal/TerminalSurface";
import { Action, DialogKeys, IconButton } from "./ui/controls";
import { Icon } from "./ui/icons";
import { Sidebar, type SidebarActions } from "./ui/Sidebar";
import { createWorkspace } from "./ui/workspace";

type Editor = { title: string; value: string; save: (value: string) => void };

function App() {
  const store = createWorkspace();
  const win = currentWindow();
  const editor = signal<Editor | null>(null);
  const draft = signal("");
  const searching = signal(false);
  const query = signal("");
  const settings = signal(false);
  const appearance = signal<"system" | "light" | "dark">("system");
  const closingGroup = signal<string | null>(null);
  const editingGroup = signal<string | null>(null);
  const version = Runtime.version();
  const compactContent = signal(false);
  const sidebarHovered = signal(false);
  const sidebarLayoutWidth = signal(store.sidebarWidth());
  const sidebarOpen = () => store.expanded() || sidebarHovered() || editingGroup() !== null;
  function toggleSidebar() {
    sidebarHovered.set(false);
    store.expanded.update((value) => !value);
  }

  win.set({
    title: "Termlevel", titleBar: { style: "hidden", height: token("height-titlebar") },
    minSize: { width: 640, height: 420 },
  });
  effect(() => win.set({ title: `${store.activeTab()?.title ?? "Termlevel"} — Termlevel` }));

  function edit(title: string, value: string, save: (value: string) => void) {
    draft.set(value);
    editor.set({ title, value, save });
  }
  function saveEditor() {
    const value = draft().trim();
    if (!value) return;
    editor()?.save(value);
    editor.set(null);
  }
  const actions: SidebarActions = {
    renameTab: (id) => edit("Rename terminal", store.tab(id)?.title ?? "", (title) => store.patchTab(id, { title })),
    newGroup: () => editingGroup.set(store.addGroup("")),
    closeGroup: closingGroup.set,
    search: () => { query.set(""); searching.set(true); },
    settings: () => settings.set(true),
    toggleSidebar,
  };

  // Application shortcuts use Command on macOS, preserving shell Control keys.
  const shortcuts: Record<string, () => void> = {
    "mod+t": () => { store.addTab(); },
    "mod+w": () => { const id = store.activeId(); if (id) store.closeTab(id); },
    "mod+b": toggleSidebar,
    "mod+shift+g": actions.newGroup,
    "mod+k": actions.search,
    "mod+,": actions.settings,
    "ctrl+tab": () => store.step(1),
    "ctrl+shift+tab": () => store.step(-1),
  };
  for (const [key, handler] of Object.entries(shortcuts)) keymap.bind(key, handler, { when: "always" });
  for (let n = 1; n <= 9; n++) keymap.bind(`mod+${n}`, () => {
    const tab = store.tabs()[n - 1];
    if (tab) store.select(tab.id);
  }, { when: "always" });

  Menu.setApplicationMenu([
    platform.os === "macos" && { label: "Termlevel", submenu: [
      { label: "About Termlevel", onSelect: actions.settings },
      { label: "Settings…", shortcut: "mod+,", onSelect: actions.settings },
      { type: "separator" }, { role: "hide" }, { role: "hideOthers" }, { role: "showAll" },
      { type: "separator" }, { role: "quit" },
    ] },
    { label: "File", submenu: [
      { label: "New terminal", shortcut: "mod+t", onSelect: () => store.addTab() },
      { label: "New group…", shortcut: "mod+shift+g", onSelect: actions.newGroup },
      { type: "separator" },
      { label: "Close terminal", shortcut: "mod+w", onSelect: shortcuts["mod+w"] },
    ] },
    { label: "Edit", submenu: [{ role: "undo" }, { role: "redo" }, { type: "separator" }, { role: "cut" }, { role: "copy" }, { role: "paste" }, { role: "selectAll" }] },
    { label: "View", submenu: [
      { label: "Toggle sidebar", shortcut: "mod+b", onSelect: shortcuts["mod+b"] },
      { label: "Find a terminal…", shortcut: "mod+k", onSelect: actions.search },
      { type: "separator" }, { role: "toggleFullScreen" },
    ] },
    { label: "Window", submenu: [{ role: "minimize" }, { role: "zoom" }, { role: "close" }] },
  ]);

  const searchResults = () => store.tabs().filter((tab) =>
    `${tab.title} ${tab.cwd} ${store.groupTitle(tab.groupId)}`.toLowerCase().includes(query().toLowerCase()));

  function SessionToolbar(props: { collapsed: boolean }) {
    return <View testID="session-toolbar" desktop={{ windowDrag: true }}
      class="h-(--height-titlebar) shrink-0 flex-row items-center gap-2 border-b border-border bg-terminal-background"
      style={{
        paddingLeft: props.collapsed ? titleBar().left + 8 : 12,
        paddingRight: titleBar().right + 8,
      }}>
      <If when={props.collapsed}>
        <View desktop={{ windowDrag: false }}>
          <IconButton icon="sidebar" label="Expand sidebar" testID="toggle-sidebar" onPress={actions.toggleSidebar} />
        </View>
      </If>
      <Text testID="toolbar-title" class="min-w-0 text-(length:--text-body) font-medium text-foreground"
        style={{ numberOfLines: 1 }}>{store.activeTab()?.title ?? "New session"}</Text>
      <View class="min-w-0 flex-1" />
      <If when={!!store.activeTab() && !compactContent()}>
        <View testID="session-path" class="min-w-0 max-w-[280px] flex-row items-center gap-1.5">
          <Text class="min-w-0 font-mono text-(length:--text-caption) text-muted-foreground"
            style={{ numberOfLines: 1 }}>{store.activeTab()?.cwd}</Text>
        </View>
      </If>
      <View desktop={{ windowDrag: false }} class="flex-row items-center gap-0.5">
        <IconButton icon="plus" label="New terminal" testID="titlebar-new-terminal" onPress={() => store.addTab()} />
        <If when={!!store.activeTab()}>
          <IconButton icon="more" label="Terminal actions" testID="terminal-actions" onPress={() => {
            const id = store.activeId();
            if (!id) return;
            void Menu.popup([
              { label: "Rename terminal…", onSelect: () => actions.renameTab(id) },
              { label: "Move to group", submenu: [
                { label: "No group", onSelect: () => store.moveTab(id, null) },
                ...store.groups().map((g) => ({ label: store.groupTitle(g.id), onSelect: () => store.moveTab(id, g.id) })),
              ] },
              { label: "Close terminal", onSelect: () => store.closeTab(id) },
            ]);
          }} />
        </If>
      </View>
    </View>;
  }

  return <View class="relative flex-1 bg-terminal-background font-sans text-foreground">
    <If when={!store.expanded()}><SessionToolbar collapsed /></If>
    <ResizablePanelGroup class="min-h-0 flex-1"
      disabled={!store.expanded()}
      handleClass={store.expanded() ? "bg-sidebar-border group-hover:bg-ring" : "bg-sidebar-border"}
      handleAccessibilityLabel="Resize terminal sidebar">
      <ResizablePanel fixed size={store.expanded() ? store.sidebarWidth() : token("width-sidebar-collapsed")}
        minSize={store.expanded() ? 176 : token("width-sidebar-collapsed")}
        maxSize={store.expanded() ? 288 : token("width-sidebar-collapsed")}
        onResize={({ size }) => { if (store.expanded()) store.sidebarWidth.set(size); }}>
        {/* Measure native layout during drag; onResize only persists the final size. */}
        <View testID="sidebar-allocation" class="h-full w-full"
          onLayout={({ bounds }) => sidebarLayoutWidth.set(bounds.size.width)} />
      </ResizablePanel>
      <ResizablePanel minSize={320}>
        <View testID="session-panel" class="h-full min-w-0 flex-1 bg-terminal-background"
          onLayout={({ bounds }) => compactContent.set(bounds.size.width < 520)}>
          <If when={store.expanded()}><SessionToolbar collapsed={false} /></If>

          {/* Stable ID keys keep every terminal mounted across selection,
              group collapse, rename and moves. Only close disposes a surface. */}
          <View class="relative min-h-0 flex-1 overflow-hidden">
            <Each of={store.tabs().map((t) => t.id)}>{(id) => <TerminalSurface id={id()} workspace={store} />}</Each>
            <If when={store.tabs().length === 0}>
              <View testID="empty-state" class="flex-1 items-center justify-center gap-3">
                <View class="h-10 w-10 items-center justify-center rounded-(--radius-panel) border border-border bg-muted">
                  <Icon name="terminal" size={20} color={token("icon-active")} />
                </View>
                <Text class="text-(length:--text-heading) font-medium">Room for something new.</Text>
                <Text class="text-(length:--text-body) text-muted-foreground">Open a terminal to get started.</Text>
                <Action primary onPress={() => { store.addTab(null); }} testID="empty-new-terminal">New terminal</Action>
              </View>
            </If>
          </View>

        </View>
      </ResizablePanel>
    </ResizablePanelGroup>

    {/* Window-level presentation escapes the narrow panel's native clipping. */}
    <View testID="sidebar-panel" class="absolute bottom-0 left-0 bg-sidebar"
      onMouseEnter={() => { if (!store.expanded()) sidebarHovered.set(true); }}
      onMouseLeave={() => sidebarHovered.set(false)}
      style={{
        top: store.expanded() ? 0 : token("height-titlebar"),
        width: store.expanded() ? sidebarLayoutWidth() : sidebarOpen() ? store.sidebarWidth() : token("width-sidebar-collapsed"),
        zIndex: 10,
        borderRightWidth: !store.expanded() && sidebarOpen() ? 1 : 0,
        borderColor: token("sidebar-border"),
        boxShadow: !store.expanded() && sidebarOpen() ? [{ x: 6, y: 0, blur: 18, color: token("panel-shadow") }] : [],
      }}>
      <Sidebar workspace={store} actions={actions} groupEditor={editingGroup} expanded={sidebarOpen} />
    </View>

    <Dialog testID="name-dialog" open={editor() !== null} animation="fade"
      accessibilityLabel={editor()?.title ?? "Edit name"} onOpenChange={({ open }) => { if (!open) editor.set(null); }}
      class="m-auto w-[320px] gap-3 rounded-(--radius-dialog) border border-border bg-popover p-4" backdropClass="bg-scrim">
      <DialogKeys dismiss={() => editor.set(null)} />
      <Text class="text-(length:--text-title) font-semibold text-foreground">{editor()?.title}</Text>
      <Input testID="name-input" value={draft()} onChange={({ value }) => draft.set(value)}
        onSubmit={saveEditor} autoFocus placeholder="Name" maxLength={80}
        class="h-(--height-input) rounded-(--radius-control) border border-input bg-background px-3 text-(length:--text-body) text-foreground" />
      <View class="flex-row justify-end gap-2">
        <Action onPress={() => editor.set(null)}>Cancel</Action>
        <Action primary testID="save-name" onPress={saveEditor}>Save</Action>
      </View>
    </Dialog>

    <Dialog testID="search-dialog" open={searching()} animation="fade" accessibilityLabel="Find a terminal"
      onOpenChange={({ open }) => searching.set(open)}
      class="m-auto w-[380px] gap-2 rounded-(--radius-dialog) border border-border bg-popover p-4" backdropClass="bg-scrim">
      <DialogKeys dismiss={() => searching.set(false)} />
      <Input testID="terminal-search" value={query()} onChange={({ value }) => query.set(value)}
        autoFocus placeholder="Find a terminal…" onSubmit={() => {
          const tab = searchResults()[0];
          if (tab) { store.select(tab.id); searching.set(false); }
        }} class="h-(--height-input) rounded-(--radius-control) border border-input bg-background px-3 text-(length:--text-body) text-foreground" />
      <Each of={searchResults()} key={(tab) => tab.id}>{(tab) =>
        <Button testID={`search-${tab().id}`} onPress={() => { store.select(tab().id); searching.set(false); }}
          class="h-(--height-tab) flex-row items-center gap-2 rounded-(--radius-control) px-3 hover:bg-control-hover focus-visible:ring-2 ring-ring">
          <Icon name="terminal" size={14} />
          <Text class="flex-1 text-(length:--text-body) text-foreground">{tab().title}</Text>
          <Text class="text-(length:--text-caption) text-muted-foreground">{store.groupTitle(tab().groupId)}</Text>
        </Button>
      }</Each>
      <If when={searchResults().length === 0}><Text class="p-3 text-(length:--text-body) text-muted-foreground">No matching terminals.</Text></If>
    </Dialog>

    <Dialog open={closingGroup() !== null} animation="fade" accessibilityLabel="Delete group"
      onOpenChange={({ open }) => { if (!open) closingGroup.set(null); }}
      class="m-auto w-[340px] gap-3 rounded-(--radius-dialog) border border-border bg-popover p-4" backdropClass="bg-scrim">
      <DialogKeys dismiss={() => closingGroup.set(null)} />
      <Text class="text-(length:--text-title) font-semibold text-foreground">Delete {store.groupTitle(closingGroup())}?</Text>
      <Text class="text-(length:--text-body) text-muted-foreground">All terminals in this group will close. Their preview output will be discarded.</Text>
      <View class="flex-row justify-end gap-2">
        <Action onPress={() => closingGroup.set(null)}>Cancel</Action>
        <Action destructive testID="confirm-close-group" onPress={() => {
          const id = closingGroup(); if (id) store.closeGroup(id); closingGroup.set(null);
        }}>Delete group</Action>
      </View>
    </Dialog>

    <Dialog testID="settings-dialog" open={settings()} animation="fade" accessibilityLabel="Settings"
      onOpenChange={({ open }) => settings.set(open)}
      class="m-auto w-[360px] gap-3 rounded-(--radius-dialog) border border-border bg-popover p-4" backdropClass="bg-scrim">
      <DialogKeys dismiss={() => settings.set(false)} />
      <View class="flex-row items-center justify-between">
        <Text class="text-(length:--text-title) font-semibold text-foreground">Settings</Text>
        <IconButton icon="close" label="Close settings" onPress={() => settings.set(false)} />
      </View>
      <View class="gap-2">
        <Text class="text-(length:--text-body) font-medium text-foreground">Appearance</Text>
        <View class="flex-row gap-1 rounded-(--radius-tab) border border-border bg-background p-1">
          <Each of={["system", "light", "dark"] as const}>{(mode) =>
            <Button
              testID={`appearance-${mode()}`}
              accessibilityLabel={`${mode()} appearance`}
              selected={appearance() === mode()}
              onPress={() => { appearance.set(mode()); setColorScheme(mode()); }}
              class="h-(--height-control) flex-1 items-center justify-center rounded-(--radius-control) hover:bg-control-hover focus-visible:ring-2 ring-ring"
              classList={{ "bg-control-selected": appearance() === mode() }}
            >
              <Text class="text-(length:--text-label) font-medium"
                style={{ color: token(appearance() === mode() ? "control-selected-foreground" : "muted-foreground") }}>
                {mode() === "system" ? "System" : mode() === "light" ? "Light" : "Dark"}
              </Text>
            </Button>
          }</Each>
        </View>
      </View>
      <View class="flex-row items-center gap-2">
        <Text class="flex-1 text-(length:--text-body) text-foreground">Terminal font size</Text>
        <Action onPress={() => store.fontSize.update((n) => Math.max(10, n - 1))}>−</Action>
        <Text class="text-(length:--text-body) text-foreground">{store.fontSize()} pt</Text>
        <Action onPress={() => store.fontSize.update((n) => Math.min(24, n + 1))}>+</Action>
      </View>
      <View class="h-px bg-border" />
      <Text class="text-(length:--text-body) text-foreground">Termlevel {version}</Text>
      <Text class="text-(length:--text-label) text-muted-foreground">A terminal workspace inspired by Firefox vertical tabs. This is an interactive UI preview; sessions and settings reset when the app restarts.</Text>
      <View class="gap-2">
        <Each of={[
          { label: "New terminal", keys: "mod+t" },
          { label: "Find terminal", keys: "mod+k" },
          { label: "Toggle sidebar", keys: "mod+b" },
          { label: "Next terminal", keys: "ctrl+tab" },
        ]}>{(shortcut) => <View class="flex-row items-center justify-between">
          <Text class="text-(length:--text-label) text-muted-foreground">{shortcut().label}</Text>
          <Text class="font-mono text-(length:--text-caption) text-subtle-foreground">{keymap.label(shortcut().keys)}</Text>
        </View>}</Each>
      </View>
    </Dialog>
  </View>;
}

mount(App);
