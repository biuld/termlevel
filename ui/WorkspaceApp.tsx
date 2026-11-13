import { effect, keymap, platform, Scroll, signal, Text, token, View } from "nsh";
import { Button, Input, ResizablePanel, ResizablePanelGroup } from "nsh/base";
import { Application, currentWindow, Menu, titleBar } from "nsh/desktop";
import { Runtime, Sessions, TerminalEngine } from "termlevel/native";
import { TerminalSurface } from "../terminal/TerminalSurface";
import { Action, AppDialog, DialogActions, Hint, IconButton } from "./controls";
import { Icon } from "./icons";
import { SettingsDialog } from "./SettingsDialog";
import { Sidebar, type SidebarActions } from "./Sidebar";
import { Type } from "./type";
import { createWorkspace } from "./workspace";

type Editor = { title: string; value: string; save: (value: string) => void };

export function WorkspaceApp(props: { shell?: string } = {}) {
  const preview = Runtime.previewMode();
  const store = createWorkspace(preview ? undefined : {
    cwd: Runtime.cwd(), list: Sessions.list,
    open: (cwd) => Sessions.open(cwd, token("terminal-foreground"), token("terminal-background"), props.shell),
    close: Sessions.close,
  });
  const win = currentWindow();
  if (!preview) {
    win.on("close", () => Sessions.shutdown());
    Application.on("quit", async () => {
      Sessions.shutdown();
      // NSH allows at most 200 ms for quit cleanup. Children get a brief
      // hangup grace period in Rust; wait for reaping within that budget.
      for (let attempt = 0; attempt < 30 && !Sessions.settled(); attempt++) {
        await new Promise<void>((resolve) => setTimeout(resolve, 5));
      }
    });
  }
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
  const sidebarLayoutWidth = signal(store.sidebarWidth() + token("gap-pane") / 2);
  const sidebarOpen = () => store.expanded() || sidebarHovered() || editingGroup() !== null;
  const sidebarFloating = () => !store.expanded() && sidebarOpen();
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
    renameTab: (id) => edit("Rename terminal", store.tab(id)?.title ?? "", (title) => store.patchTab(id, { title, customTitle: true })),
    newGroup: () => editingGroup.set(store.addGroup()),
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

  function SessionToolbar() {
    const showPath = () => !!store.activeTab() && !compactContent();
    return <View testID="session-toolbar"
      class="h-(--height-toolbar) shrink-0 flex-row items-center bg-terminal-background"
      style={{ paddingLeft: token("padding-terminal"), paddingRight: token("gap-pane") }}>
      <View class="min-w-0 flex-1 flex-row items-center gap-2">
        <Icon name="terminal" size={14} />
        <If when={showPath()}>
          <Type testID="session-path" role="body" family="mono" tone="muted-foreground" lines={1} class="min-w-0 flex-1"
            tooltip={<Hint>{`${preview ? "Demo directory" : "Initial directory"}: ${store.activeTab()?.cwd}`}</Hint>}>{store.activeTab()?.cwd}</Type>
        </If>
        <If when={!showPath()}>
          <Type role="body" tone="muted-foreground" lines={1} class="min-w-0 flex-1">{store.activeTab()?.title ?? "Terminal"}</Type>
        </If>
      </View>
      <View desktop={{ windowDrag: false }}
        class="h-(--height-control) flex-row items-center gap-0.5">
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

  return <TerminalEngine class="relative flex-1 bg-chrome font-sans text-foreground">
    <View testID="window-toolbar" desktop={{ windowDrag: true }}
      class="relative h-(--height-titlebar) shrink-0 flex-row items-center"
      style={{ paddingLeft: titleBar().left + token("gap-pane"), paddingRight: titleBar().right + token("gap-pane") }}>
      <View desktop={{ windowDrag: false }}>
        <IconButton circle icon="sidebar" label={store.expanded() ? "Collapse sidebar" : "Expand sidebar"}
          testID="toggle-sidebar" onPress={actions.toggleSidebar} />
      </View>
      <View class="absolute inset-0 items-center justify-center" style={{ pointerEvents: "none" }}>
        <Type role="label">Termlevel</Type>
      </View>
    </View>
    <View class="min-h-0 flex-1 px-(--gap-pane) pb-(--gap-pane)">
      <ResizablePanelGroup class="min-h-0 flex-1"
        disabled={!store.expanded()}
        handleClass={store.expanded() ? "bg-transparent group-hover:bg-sidebar-border group-active:bg-ring" : "bg-transparent"}
        handleAccessibilityLabel="Resize terminal sidebar">
        <ResizablePanel fixed size={(store.expanded() ? store.sidebarWidth() : token("width-sidebar-collapsed")) + token("gap-pane") / 2}
          minSize={(store.expanded() ? 176 : token("width-sidebar-collapsed")) + token("gap-pane") / 2}
          maxSize={(store.expanded() ? 288 : token("width-sidebar-collapsed")) + token("gap-pane") / 2}
          onResize={({ size }) => { if (store.expanded()) store.sidebarWidth.set(size - token("gap-pane") / 2); }}>
          {/* Measure native layout during drag; onResize only persists the final size. */}
          <View testID="sidebar-allocation" class="h-full w-full"
            onLayout={({ bounds }) => sidebarLayoutWidth.set(bounds.size.width)} />
        </ResizablePanel>
        <ResizablePanel minSize={320}>
          <View class="h-full min-w-0 flex-1" style={{ paddingLeft: token("gap-pane") / 2 }}>
            <View testID="session-panel" class="min-h-0 min-w-0 flex-1 overflow-hidden rounded-(--radius-panel) bg-terminal-background"
              onLayout={({ bounds }) => compactContent.set(bounds.size.width < 520)}>
              <SessionToolbar />

              {/* Stable ID keys keep every terminal mounted across selection,
                  group collapse, rename and moves. Only close disposes a surface. */}
              <View class="relative min-h-0 flex-1 overflow-hidden">
                <Each of={store.tabs().map((t) => t.id)}>{(id) => <TerminalSurface id={id()} workspace={store} preview={preview} />}</Each>
                <If when={store.tabs().length === 0}>
                  <View testID="empty-state" class="flex-1 items-center justify-center gap-3">
                    <View class="h-10 w-10 items-center justify-center rounded-(--radius-panel) border border-border bg-muted">
                      <Icon name="terminal" size={20} color={token("icon-active")} />
                    </View>
                    <Type role="heading">Room for something new.</Type>
                    <Type role="body" tone="muted-foreground">Open a terminal to get started.</Type>
                    <Action primary onPress={() => { store.addTab(null); }} testID="empty-new-terminal">New terminal</Action>
                  </View>
                </If>
              </View>

            </View>
          </View>
        </ResizablePanel>
      </ResizablePanelGroup>
    </View>

    {/* Window-level presentation escapes the narrow panel's native clipping. */}
    <View testID="sidebar-panel" class="absolute"
      onMouseEnter={() => { if (!store.expanded()) sidebarHovered.set(true); }}
      onMouseLeave={() => sidebarHovered.set(false)}
      style={{
        top: token("height-titlebar"),
        bottom: token("gap-pane"),
        left: token("gap-pane"),
        width: store.expanded() ? sidebarLayoutWidth() - token("gap-pane") / 2 : sidebarOpen() ? store.sidebarWidth() : token("width-sidebar-collapsed"),
        zIndex: 10,
        // All navigation states share one surface; elevation signals an overlay.
        backgroundColor: token("sidebar"),
        borderWidth: sidebarFloating() ? 1 : 0,
        borderColor: token("sidebar-border"),
        borderRadius: token("radius-panel"),
        boxShadow: sidebarFloating() ? [{ x: 6, y: 0, blur: 28, color: token("panel-shadow") }] : [],
      }}>
      <Sidebar workspace={store} actions={actions} groupEditor={editingGroup} expanded={sidebarOpen} />
    </View>

    <AppDialog testID="name-dialog" open={editor() !== null} label={editor()?.title ?? "Edit name"} title={editor()?.title}
      frame="w-[320px]" dismiss={() => editor.set(null)}>
      <Input testID="name-input" value={draft()} onChange={({ value }) => draft.set(value)}
        onSubmit={saveEditor} autoFocus placeholder="Name" maxLength={80}
        class="h-(--height-input) rounded-(--radius-control) border border-input bg-background px-3 text-(length:--text-body) text-foreground" />
      <DialogActions>
        <Action onPress={() => editor.set(null)}>Cancel</Action>
        <Action primary testID="save-name" onPress={saveEditor}>Save</Action>
      </DialogActions>
    </AppDialog>

    <AppDialog testID="search-dialog" open={searching()} label="Find a terminal" title="Find a terminal"
      frame="w-[380px]" dismiss={() => searching.set(false)}>
      <Input testID="terminal-search" value={query()} onChange={({ value }) => query.set(value)}
        autoFocus placeholder="Name or group" onSubmit={() => {
          const tab = searchResults()[0];
          if (tab) { store.select(tab.id); searching.set(false); }
        }} class="h-(--height-input) rounded-(--radius-control) border border-input bg-background px-3 text-(length:--text-body) text-foreground" />
      <Scroll class="max-h-[240px]">
        <Each of={searchResults()} key={(tab) => tab.id}>{(tab) =>
          <Button testID={`search-${tab().id}`} onPress={() => { store.select(tab().id); searching.set(false); }}
            class="h-(--height-tab) flex-row items-center gap-2 rounded-(--radius-control) px-2 hover:bg-control-hover focus-visible:ring-2 ring-ring">
            <Icon name="terminal" size={14} />
            <Text class="min-w-0 flex-1 text-(length:--text-body) text-foreground" style={{ numberOfLines: 1 }}>{tab().title}</Text>
            <Text class="text-(length:--text-caption) text-muted-foreground">{store.groupTitle(tab().groupId)}</Text>
          </Button>
        }</Each>
        <If when={searchResults().length === 0}><Text class="px-2 py-3 text-(length:--text-body) text-muted-foreground">No matching terminals.</Text></If>
      </Scroll>
    </AppDialog>

    <AppDialog open={closingGroup() !== null} role="alertdialog" label="Delete group"
      title={closingGroup() ? `Delete ${store.groupTitle(closingGroup())}?` : undefined}
      description={preview ? "All terminals in this group will close. Their preview output will be discarded." : "All terminals in this group will close and their running processes will end."}
      frame="w-[340px]" dismiss={() => closingGroup.set(null)}>
      <DialogActions>
        <Action onPress={() => closingGroup.set(null)}>Cancel</Action>
        <Action destructive testID="confirm-close-group" onPress={() => {
          const id = closingGroup(); if (id) store.closeGroup(id); closingGroup.set(null);
        }}>Delete group</Action>
      </DialogActions>
    </AppDialog>

    <SettingsDialog open={settings()} dismiss={() => settings.set(false)} appearance={appearance}
      fontSize={store.fontSize} version={version} preview={preview} />
  </TerminalEngine>;
}

