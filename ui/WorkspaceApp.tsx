import { effect, keymap, platform, Scroll, signal, Text, token, View } from "nsh";
import { Button, Input, ResizablePanel, ResizablePanelGroup } from "nsh/base";
import { Application, currentWindow, Menu, titleBar } from "nsh/desktop";
import { Runtime, Sessions, TerminalEngine } from "termlevel/native";
import { TilingCanvas } from "./TilingCanvas";
import { Action, AppDialog, DialogActions, fieldClass, IconButton } from "./controls";
import { Icon } from "./icons";
import { GroupPathDialog } from "./GroupPathDialog";
import { SettingsDialog } from "./SettingsDialog";
import { Sidebar, type SidebarActions } from "./Sidebar";
import { Type } from "./type";
import { applyFonts } from "./fonts";
import { createWorkspace, type Workspace } from "./workspace";

type Editor = { title: string; value: string; save: (value: string) => void };

export function WorkspaceApp(props: { shell?: string; workspace?: Workspace } = {}) {
  const preview = Runtime.previewMode();
  const store = props.workspace ?? createWorkspace(preview ? undefined : {
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
  const finding = signal(false);
  const findQuery = signal("");
  const appearance = signal<"system" | "light" | "dark">("system");
  effect(() => { applyFonts(); });
  const closingGroup = signal<string | null>(null);
  const editingGroup = signal<string | null>(null);
  const choosingPath = signal(false);
  const pathError = signal("");
  const version = Runtime.version();
  effect(() => {
    const modal = editor() !== null || searching() || settings() || closingGroup() !== null || editingGroup() !== null || choosingPath() || finding();
    if (modal) { store.resizing.set(false); store.cancelPaneDrag(); }
    store.inputBlocked.set(modal || store.resizing());
  });
  const terminalAction = (action: () => void) => () => { if (!store.inputBlocked()) action(); };
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
    newGroup: () => { pathError.set(""); choosingPath.set(true); },
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
    "mod+d": terminalAction(() => { store.splitTerminal("right"); }),
    "mod+shift+d": terminalAction(() => { store.splitTerminal("down"); }),
    "mod+shift+enter": terminalAction(store.toggleZoom),
    "mod+shift+g": actions.newGroup,
    "mod+k": actions.search,
    "mod+f": () => { if (editor() === null && !searching() && !settings() && closingGroup() === null && editingGroup() === null && !choosingPath()) finding.set(true); },
    "mod+,": actions.settings,
    "ctrl+tab": () => store.step(1),
    "ctrl+shift+tab": () => store.step(-1),
  };
  for (const [key, handler] of Object.entries(shortcuts)) keymap.bind(key, handler, { when: "always" });
  for (const [key, direction] of [["left", "left"], ["right", "right"], ["up", "up"], ["down", "down"]] as const) {
    keymap.bind(`mod+alt+${key}`, terminalAction(() => store.focusDirection(direction)), { when: "always" });
    keymap.bind(`mod+alt+shift+${key}`, terminalAction(() => store.focusDirection(direction, true)), { when: "always" });
  }
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
      { label: "Find in terminal…", shortcut: "mod+f", onSelect: () => finding.set(true) },
      { type: "separator" },
      { label: "Split right", shortcut: "mod+d", onSelect: shortcuts["mod+d"] },
      { label: "Split down", shortcut: "mod+shift+d", onSelect: shortcuts["mod+shift+d"] },
      { label: "Maximize / restore pane", shortcut: "mod+shift+enter", onSelect: shortcuts["mod+shift+enter"] },
      { label: "Hide pane", onSelect: () => store.hidePane() },
      { label: "Equalize layout", onSelect: store.equalizeLayout },
      { type: "separator" }, { role: "toggleFullScreen" },
    ] },
    { label: "Window", submenu: [{ role: "minimize" }, { role: "zoom" }, { role: "close" }] },
  ]);

  const searchResults = () => store.tabs().filter((tab) =>
    `${tab.title} ${tab.cwd} ${store.groupTitle(tab.groupId)}`.toLowerCase().includes(query().toLowerCase()));

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
      <View testID="global-actions" desktop={{ windowDrag: false }} class="ml-auto flex-row items-center gap-0.5">
        <IconButton icon="folder" label="New group" testID="new-group" onPress={actions.newGroup} />
        <IconButton icon="search" label="Find a terminal" testID="find-terminal" onPress={actions.search} />
        <IconButton icon="settings" label="Settings" testID="settings" onPress={actions.settings} />
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
            <TilingCanvas workspace={store} preview={preview} rename={actions.renameTab} finding={finding} findQuery={findQuery} />
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
        class={fieldClass} />
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
        }} class={fieldClass} />
      <Scroll class="max-h-[240px]">
        <Each of={searchResults()} key={(tab) => tab.id}>{(tab) =>
          <Button testID={`search-${tab().id}`} onPress={() => { store.select(tab().id); searching.set(false); }}
            class="h-(--height-tab) flex-row items-center gap-2 rounded-(--radius-tab) px-2 hover:bg-tab-hover focus-visible:ring-2 ring-sidebar-ring">
            <Icon name="terminal" size={13} color={token("icon")} />
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

    <GroupPathDialog open={choosingPath()} error={pathError()} dismiss={() => choosingPath.set(false)}
      paths={[...new Set([store.activeTab()?.cwd, ...store.tabs().map((tab) => tab.cwd)].filter((cwd): cwd is string => !!cwd))]}
      onChoose={(cwd) => { choosingPath.set(false); store.addGroup(cwd); }}
      onBrowse={() => {
        const start = store.activeTab()?.cwd ?? "";
        void Runtime.pickFolder(start).then((picked) => {
          if (!choosingPath()) return;
          if (picked) { choosingPath.set(false); store.addGroup(picked); }
        }).catch((error: unknown) => {
          pathError.set(error instanceof Error ? error.message : "Could not open the folder dialog.");
        });
      }} />
    <SettingsDialog open={settings()} dismiss={() => settings.set(false)} appearance={appearance}
      fontSize={store.fontSize} version={version} preview={preview} scrollback={store.scrollback} clipboardRead={store.clipboardRead}
      scrollbackMin={Runtime.scrollbackMin()} scrollbackMax={Runtime.scrollbackMax()} scrollbackStep={Runtime.scrollbackStep()} />
  </TerminalEngine>;
}

