import { cleanup, token, View, type GestureResponderEvent, type Signal, type ViewRef } from "nsh";
import { Menu } from "nsh/desktop";
import { ResizeKeys, TerminalMetrics } from "termlevel/native";
import { monoFont } from "../shared/fonts";
import { TerminalSurface } from "../terminal/TerminalSurface";
import { Action, DialogKeys, Hint, IconButton } from "../shared/controls";
import { Icon } from "../shared/icons";
import { Type } from "../shared/type";
import { groupColor } from "../shared/theme";
import type { Workspace } from "./workspace";
import type { Divider, Direction, Rect } from "./tiling";

export function TilingCanvas(props: {
  workspace: Workspace;
  preview: boolean;
  rename: (id: string) => void;
  newGroup: () => void;
  finding: Signal<boolean>;
  findQuery: Signal<string>;
}) {
  const store = props.workspace;
  let canvas: ViewRef | undefined;
  let measuring = false;
  let disposed = false;
  cleanup(() => { disposed = true; });
  function measureOrigin() {
    if (measuring) return;
    measuring = true;
    void canvas?.measure().then(frame => {
      measuring = false;
      if (!disposed && frame) store.canvasOrigin.set(frame.pagePosition);
    });
  }
  function menu(id: string) {
    store.select(id);
    const existing = (direction: Direction) => store.tabs()
      .filter(tab => (tab.groupId ?? "") === store.workspaceKey())
      .map(tab => ({ label: tab.title, onSelect: () => store.splitTerminal(direction, tab.id) }));
    void Menu.popup([
      { label: "Split right", shortcut: "mod+d", disabled: !store.canSplit("right"), onSelect: () => store.splitTerminal("right") },
      { label: "Split down", shortcut: "mod+shift+d", disabled: !store.canSplit("down"), onSelect: () => store.splitTerminal("down") },
      { label: "Show terminal on the right", disabled: !store.canSplit("right"), submenu: existing("right") },
      { label: "Show terminal below", disabled: !store.canSplit("down"), submenu: existing("down") },
      { type: "separator" },
      { label: "Focus pane", disabled: store.paneList().length < 2, submenu: (["left", "right", "up", "down"] as const).map(direction => ({
        label: direction[0].toUpperCase() + direction.slice(1), shortcut: `mod+alt+${direction}`, onSelect: () => store.focusDirection(direction),
      })) },
      { label: "Swap with pane", disabled: store.paneList().length < 2, submenu: (["left", "right", "up", "down"] as const).map(direction => ({
        label: direction[0].toUpperCase() + direction.slice(1), shortcut: `mod+alt+shift+${direction}`, onSelect: () => store.focusDirection(direction, true),
      })) },
      { label: store.tiling().zoomed ? "Restore layout" : "Maximize pane", disabled: store.paneList().length < 2, onSelect: store.toggleZoom },
      { label: "Equalize layout", disabled: store.paneList().length < 2, onSelect: store.equalizeLayout },
      { label: "Resize with arrow keys", disabled: store.paneList().length < 2 || store.constrained() || store.tiling().zoomed, onSelect: () => store.resizing.set(true) },
      { label: "Hide pane", disabled: store.paneList().length < 2, onSelect: () => store.hidePane() },
      { type: "separator" },
      { label: "Find in terminal…", shortcut: "mod+f", onSelect: () => props.finding.set(true) },
      { label: "Rename terminal…", onSelect: () => props.rename(id) },
      { label: store.inSplit(id) ? "Move split to group" : "Move to group", submenu: [
        { label: "No group", disabled: store.inSplit(id) && !store.tab(id)?.groupId, onSelect: () => store.moveTab(id, null) },
        ...store.groups().map(g => ({
          label: store.groupTitle(g.id), disabled: store.inSplit(id) && store.tab(id)?.groupId === g.id,
          onSelect: () => store.moveTab(id, g.id),
        })),
      ] },
      { label: "Close terminal", onSelect: () => store.closeTab(id) },
    ]);
  }
  function SessionPane(row: { id: string }) {
    const id = row.id;
    const tab = () => store.tab(id);
    const focused = () => store.activeId() === id;
    const shown = () => !!store.frameForSession(id);
    let lastFrame: Rect = { x: 0, y: 0, width: 640, height: 400 };
    const frame = () => { const next = store.frameForSession(id); if (next) lastFrame = next; return lastFrame; };
    const showPath = () => store.paneList().length === 1 && frame().width >= 520;
    const cwd = () => tab()?.liveCwd ?? tab()?.cwd;
    const title = () => showPath() ? cwd() : tab()?.title;
    const endDrag = () => { store.finishPaneDrag(); };
    return <View testID={`pane-${id}`} class="absolute overflow-hidden rounded-(--radius-panel) bg-terminal-background"
      accessibilityElementsHidden={!shown()}
      onTouchStartCapture={() => { if (shown()) store.select(id); }}
      style={{ left: frame().x, top: frame().y, width: frame().width, height: frame().height,
        visibility: shown() ? "visible" : "hidden", opacity: shown() ? 1 : 0, pointerEvents: shown() ? "auto" : "none" }}>
      <View testID={focused() ? "session-toolbar" : `pane-toolbar-${id}`}
        class="h-(--height-toolbar) shrink-0 flex-row items-center"
        style={{ paddingLeft: token("padding-terminal"), paddingRight: token("gap-pane") }}>
        <View testID={`pane-title-${id}`} class="min-w-0 flex-1 flex-row items-center gap-2"
          panActivation={{ activeOffsetX: 8, activeOffsetY: 8 }}
          onPress={() => store.select(id)}
          onResponderGrant={() => { measureOrigin(); store.beginPaneDrag(id); }}
          onResponderMove={e => store.trackPaneDrag(e.pageX, e.pageY)}
          onTouchMove={e => { if (store.paneDrag()?.sessionId === id) store.trackPaneDrag(e.pageX, e.pageY); }}
          onResponderRelease={endDrag} onTouchEnd={endDrag}
          onResponderTerminate={store.cancelPaneDrag}
          onPanEnd={e => { if (e.cancelled) store.cancelPaneDrag(); else endDrag(); }}>
          <If when={tab()?.groupId}>
            <View accessibilityLabel={`${store.groupTitle(tab()?.groupId ?? null)} group`} class="h-2 w-2 shrink-0 rounded-sm"
              style={{ backgroundColor: groupColor(store.group(tab()?.groupId ?? "")?.color) }} />
          </If>
          <Icon name="terminal" size={14} color={token(focused() ? "icon-active" : "icon")} />
          <Type testID={showPath() && focused() ? "session-path" : `pane-name-${id}`}
            label={showPath() ? `Current directory ${cwd()}` : undefined}
            role="body" family={showPath() ? "mono" : "ui"} tone="muted-foreground" lines={1} class="min-w-0 flex-1"
            tooltip={<Hint>{`${tab()?.title} · ${store.groupTitle(tab()?.groupId ?? null)} · ${props.preview ? "Demo directory" : "Current directory"}: ${cwd()}`}</Hint>}>{title()}</Type>
          <If when={tab()?.bell}><View testID={`pane-bell-${id}`} class="h-1.5 w-1.5 rounded-full bg-primary" /></If>
        </View>
        <View class="flex-row items-center">
          <If when={tab()?.state === "exited" || tab()?.state === "error"}>
            <Type testID={`session-state-${id}`} role="caption" class="px-1">
              {tab()?.state === "exited" ? `Exited · ${tab()?.exitCode ?? "unknown"}` : "Error"}
            </Type>
            <IconButton icon="plus" label="Open new terminal" testID={`session-new-${id}`}
              onPress={() => { store.select(id); store.addTab(tab()?.groupId ?? null); }} />
          </If>
          <IconButton icon="search" label="Find in terminal" testID={focused() ? "pane-find" : `pane-find-${id}`}
            onPress={() => { store.select(id); props.finding.set(true); }} />
          <IconButton icon="splitRight" label="Split right" testID={focused() ? "split-right" : `split-right-${id}`}
            disabled={!store.canSplit("right", store.paneForSession(id)?.id)} onPress={() => { store.select(id); store.splitTerminal("right"); }} />
          <If when={frame().width >= 400}>
            <IconButton icon="splitDown" label="Split down" testID={focused() ? "split-down" : `split-down-${id}`}
              disabled={!store.canSplit("down", store.paneForSession(id)?.id)} onPress={() => { store.select(id); store.splitTerminal("down"); }} />
          </If>
          <If when={store.paneList().length > 1}>
            <IconButton icon={store.tiling().zoomed ? "restore" : "maximize"} label={store.tiling().zoomed ? "Restore layout" : "Maximize pane"}
              testID={focused() ? "zoom-pane" : `zoom-pane-${id}`} onPress={() => { store.select(id); store.toggleZoom(); }} />
          </If>
          <IconButton icon="more" label="Terminal actions" testID={focused() ? "terminal-actions" : `terminal-actions-${id}`} onPress={() => menu(id)} />
        </View>
      </View>
      <View class="relative min-h-0 flex-1"><TerminalSurface id={id} workspace={store} preview={props.preview} finding={props.finding} findQuery={props.findQuery} /></View>
      <If when={focused() && store.paneList().length > 1}>
        <View testID="focused-pane-ring" class="absolute inset-0 rounded-(--radius-panel) border border-ring" style={{ pointerEvents: "none" }} />
      </If>
    </View>;
  }
  function ResizeHandle(props: { id: string }) {
    const divider = () => store.fullGeometry().dividers.find(d => d.id === props.id)!;
    let initial: { divider: Divider; ratio: number; x: number; y: number } | null = null;
    const begin = (e: GestureResponderEvent) => {
      const d = divider();
      const extent = (d.axis === "row" ? d.parent.width : d.parent.height) - token("gap-pane");
      initial = { divider: d, ratio: (d.axis === "row" ? d.rect.x - d.parent.x : d.rect.y - d.parent.y) / extent, x: e.pageX, y: e.pageY };
    };
    const move = (e: GestureResponderEvent) => {
      if (!initial) return;
      const d = initial.divider, extent = (d.axis === "row" ? d.parent.width : d.parent.height) - token("gap-pane");
      store.resizeSplit(d.id, initial.ratio + (d.axis === "row" ? e.pageX - initial.x : e.pageY - initial.y) / extent);
    };
    return <View testID={`divider-${props.id}`} accessibilityLabel="Resize terminal panes" role="adjustable"
      class="absolute group items-center justify-center"
      accessibilityValue={{ min: 0, max: 100, now: Math.round((divider().axis === "row" ? divider().rect.x - divider().parent.x : divider().rect.y - divider().parent.y) / ((divider().axis === "row" ? divider().parent.width : divider().parent.height) - token("gap-pane")) * 100) }}
      accessibilityActions={[{ name: "increment", label: "Grow first pane" }, { name: "decrement", label: "Shrink first pane" }]}
      onAccessibilityAction={({ actionName }) => {
        const d = divider(), extent = (d.axis === "row" ? d.parent.width : d.parent.height) - token("gap-pane");
        const ratio = (d.axis === "row" ? d.rect.x - d.parent.x : d.rect.y - d.parent.y) / extent;
        store.resizeSplit(d.id, ratio + (actionName === "increment" ? 0.04 : -0.04));
      }}
      style={{ left: divider().rect.x, top: divider().rect.y, width: divider().rect.width, height: divider().rect.height,
        cursor: divider().axis === "row" ? "col-resize" : "row-resize" }}
      panActivation={{ activeOffsetX: 1, activeOffsetY: 1 }}
      onTouchStart={begin} onResponderGrant={e => { if (!initial) begin(e); }}
      onResponderMove={move} onTouchMove={move}
      onResponderRelease={() => { initial = null; }} onTouchEnd={() => { initial = null; }}
      onResponderTerminate={() => { if (initial) store.resizeSplit(props.id, initial.ratio); initial = null; }}
      onTouchCancel={() => { if (initial) store.resizeSplit(props.id, initial.ratio); initial = null; }}
      onPanEnd={e => { if (e.cancelled && initial) store.resizeSplit(props.id, initial.ratio); initial = null; }}
      onPress={e => { if (e.clickCount === 2) store.resizeSplit(props.id, 0.5); }}>
      <View class="rounded-full bg-transparent group-hover:bg-sidebar-border group-active:bg-ring"
        style={{ width: divider().axis === "row" ? 2 : "100%", height: divider().axis === "row" ? "100%" : 2 }} />
    </View>;
  }
  const hint = () => store.resizing() ? "Resize panes with arrow keys · Enter or Escape to finish"
    : store.constrained() ? "Window too small · Showing the focused terminal" : store.layoutMessage();
  return <View testID="session-panel" class="relative min-h-0 min-w-0 flex-1"
    ref={ref => { canvas = ref; measureOrigin(); }}
    onLayout={({ bounds }) => { store.setCanvasSize(bounds.size); measureOrigin(); }}>
    <TerminalMetrics fontFamily={monoFont()} fontSize={store.fontSize()} onChange={({ width }) => { if (store.cellWidth() !== width) store.cellWidth.set(width); }} />
    {/* A single keyed native surface per session survives every tree operation. */}
    <Each of={store.tabs().map(tab => tab.id)}>{id => <SessionPane id={id()} />}</Each>
    <If when={!store.tiling().zoomed && !store.constrained()}>
      <Each of={store.fullGeometry().dividers.map(d => d.id)}>{id => <ResizeHandle id={id()} />}</Each>
    </If>
    <If when={store.tabs().length === 0}>
      <View testID="empty-state" class="flex-1 items-center justify-center gap-3 rounded-(--radius-panel) bg-terminal-background">
        <View class="h-10 w-10 items-center justify-center rounded-(--radius-panel) border border-border bg-muted"><Icon name="terminal" size={20} /></View>
        <Type role="heading">Room for something new.</Type>
        <Type role="body" tone="muted-foreground">Open a terminal in your home folder, or create a group for a project.</Type>
        <View class="flex-row items-center gap-2">
          <Action primary onPress={() => store.addTab()} testID="empty-new-terminal">Open terminal in Home</Action>
          <Action onPress={() => props.newGroup()} testID="empty-new-group">New group…</Action>
        </View>
      </View>
    </If>
    <If when={store.paneDrag() !== null}><DialogKeys dismiss={store.cancelPaneDrag} /></If>
    <If when={store.resizing()}><ResizeKeys onDirection={({ direction }) => store.resizeDirection(direction as Direction)} onFinish={() => store.resizing.set(false)} /></If>
    <If when={!!hint()}>
      <View testID="layout-hint" class="absolute bottom-2 left-2 right-2 rounded-(--radius-control) border border-border bg-popover px-2 py-1 flex-row items-center">
        <Type role="caption" class="min-w-0 flex-1">{hint()}</Type>
        <If when={store.resizing() || !!store.layoutMessage()}>
          <IconButton icon="close" label="Dismiss layout message" onPress={() => { store.resizing.set(false); store.layoutMessage.set(""); }} />
        </If>
      </View>
    </If>
    <If when={store.paneDrag()?.targetId != null && store.dropPreview() != null}>
      <View testID="pane-drop-preview" class="absolute items-center justify-center rounded-(--radius-panel) border-2 bg-primary/15"
        style={{ left: store.dropPreview()!.x, top: store.dropPreview()!.y, width: store.dropPreview()!.width, height: store.dropPreview()!.height,
          borderColor: token(store.paneDrag()!.allowed ? "ring" : "destructive"), pointerEvents: "none", zIndex: 20 }}>
        <Type role="label" class="rounded-(--radius-control) bg-popover px-2 py-1">{store.paneDrag()!.allowed
          ? store.workspaceKey() === "" && store.paneDrag()!.position !== "center" ? "Add to split"
            : store.paneDrag()!.joinsGroup ? "Add to this group"
            : store.paneDrag()!.position === "center" ? "Swap or replace terminal" : "Move terminal here"
          : "Cannot place terminal here"}</Type>
      </View>
    </If>
  </View>;
}
