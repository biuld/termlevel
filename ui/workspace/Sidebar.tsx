import { reduceMotion, keymap, Scroll, signal, Text, token, View, type GestureResponderEvent, type Point, type ViewRef, type Signal } from "nsh";
import { Button, Progress } from "nsh/base";
import { Menu } from "nsh/desktop";
import { Icon, type IconName } from "../shared/icons";
import { WorkingIndicator, type WorkingIndicatorKind } from "../shared/WorkingIndicator";
import { Hint, IconButton } from "../shared/controls";
import type { Workspace } from "./state";
import { groupColor, groupLine, groupTint } from "../shared/theme";
import { GroupEditor } from "./GroupEditor";
import { Type } from "../shared/type";
import { AgentIcon, agentForApp } from "./agent-icons";

type DragSlot = { ref: ViewRef; kind: "tab" | "header" | "ungrouped"; id: string | null };
type SlotRect = { x: number; y: number; width: number; height: number };
type DropTarget = { groupId: string | null; pinned: boolean; beforeId: string | null; headerId: string | null; combineId: string | null };

export interface SidebarActions {
  newTerminal(): void;
  renameTab(id: string): void;
  newGroup(): void;
  closeGroup(id: string): void;
  search(): void;
  settings(): void;
  toggleSidebar(): void;
}

export function Sidebar(props: { workspace: Workspace; actions: SidebarActions; groupEditor: Signal<string | null>; expanded: () => boolean; indicator: () => WorkingIndicatorKind }) {
  const store = props.workspace;
  const actions = props.actions;
  const members = (groupId: string | null) => store.tabs().filter((t) => t.groupId === groupId).sort((a, b) => Number(b.pinned) - Number(a.pinned));
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
      const peers = members(slot.id).filter((t) => t.id !== sourceId && t.pinned === source.pinned);
      return { groupId: slot.id, pinned: source.pinned, beforeId: peers[0]?.id ?? null, headerId: slot.id, combineId: null };
    }
    if (slot.kind === "ungrouped") {
      const peers = members(null).filter((t) => t.id !== sourceId && t.pinned === source.pinned);
      return { groupId: null, pinned: source.pinned, beforeId: peers[0]?.id ?? null, headerId: null, combineId: null };
    }
    const target = slot.id ? store.tab(slot.id) : undefined;
    if (!target || target.id === sourceId) return null;
    const sameSection = source.groupId === target.groupId && source.pinned === target.pinned;
    const upper = pageY < top + height / 2;
    if (onTab && (source.groupId !== target.groupId || !target.groupId)) {
      if (target.groupId) {
        const peers = store.tabs().filter((t) => t.groupId === target.groupId && t.pinned === source.pinned && t.id !== sourceId);
        const index = peers.findIndex((t) => t.id === target.id);
        return { groupId: target.groupId, pinned: source.pinned, beforeId: index < 0 ? null : upper ? target.id : peers[index + 1]?.id ?? null, headerId: null, combineId: target.id };
      }
      return { groupId: null, pinned: source.pinned, beforeId: upper ? target.id : null, headerId: null, combineId: target.id };
    }
    if (!sameSection) return null;
    const peers = store.tabs().filter((t) => t.groupId === target.groupId && t.pinned === target.pinned);
    const index = peers.findIndex((t) => t.id === target.id);
    const beforeId = upper ? target.id : peers[index + 1]?.id ?? null;
    return {
      groupId: target.groupId,
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
    const active = () => store.activeId() === id && !store.canvasBlocked();
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
      return groupId ? groupColor(store.group(groupId)?.color) : null;
    };
    const workingAccent = () => {
      const groupId = tab()?.groupId;
      return groupId ? groupColor(store.group(groupId)?.color) : token("primary");
    };
    const status = () => tab()?.status;
    const progress = () => status()?.progress;
    const lineIndeterminate = () => progress()?.indeterminate ?? false;
    const animateLine = () => !!progress()?.animate && !reduceMotion();
    const working = () => status()?.state === "working";
    const agent = () => {
      if (tab()?.activity?.state !== "idle" || status()?.state !== "idle") return null;
      return agentForApp(tab()?.activity?.app);
    };
    const statusIcon = (): IconName => {
      switch (status()?.state) {
        case "done": return "check";
        case "blocked": return "alert";
        case "error": case "hostError": case "exited": return "xCircle";
        default: return tab()?.pinned ? "pin" : "terminal";
      }
    };
    const statusColor = () => {
      switch (status()?.state) {
        case "working": return workingAccent();
        case "blocked": return token("notification-warning");
        case "done": return token("notification-success");
        case "error": case "hostError": case "exited": return token("destructive");
        default: return accent() ?? token(active() ? "icon-active" : "icon");
      }
    };
    const summary = () => [tab()?.title ?? "Terminal", agent() ? "Idle" : status()?.label, progress()?.label].filter(Boolean).join(", ");
    const detail = () => [agent()?.name ?? status()?.app, status()?.detail,
      status()?.pendingCount ? `${status()!.pendingCount} more need attention` : ""].filter(Boolean).join(" · ");
    const tabLabel = () => [summary(), detail(), tab()?.bell ? "terminal bell" : ""].filter(Boolean).join(", ");
    const lineColor = () => progress()?.state === "error" ? token("destructive")
      : progress()?.state === "pause" ? token("subtle-foreground") : workingAccent();
    const progressLine = (compact: boolean) => (
      <If when={!!progress()}>
        <View class={compact ? "mt-0.5 h-0.5 w-3" : "mt-0.5 h-0.5 w-full"}
          style={{ pointerEvents: "none" }} testID={`tab-progress-${id}`}>
          <If when={progress()?.percent != null || lineIndeterminate()}>
            <Progress value={progress()?.percent ?? undefined} indeterminate={lineIndeterminate()}
              accessibilityLabel={progress()?.label}
              class="h-full w-full"
              trackClass="h-full w-full overflow-hidden rounded-full bg-sidebar-border/50"
              indicatorClass={`h-full rounded-full ${lineIndeterminate() ? "w-1/3" : ""}`}
              indicatorStyle={{ backgroundColor: lineColor(), ...(animateLine()
                ? { animationName: "pulse", animationDuration: "1.2s", animationIterationCount: "infinite" } : {}) }} />
          </If>
          <If when={progress()?.percent == null && !lineIndeterminate()}>
            <View accessibilityLabel={progress()?.label}
              class={`h-full w-full overflow-hidden rounded-full ${progress()?.state === "error" ? "bg-destructive/30" : "bg-sidebar-border/50"}`} />
          </If>
        </View>
      </If>
    );
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
        accessibilityLabel={tabLabel()} onPress={() => store.select(id)}
        tooltip={<View class="max-w-80"><Hint lines={2}>{[summary(), detail()].filter(Boolean).join("\n")}</Hint></View>}
        class="h-full min-w-0 flex-1 flex-row items-center gap-2 rounded-(--radius-tab) focus-visible:ring-2 ring-sidebar-ring"
        style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 7 : 0 }}>
        <View class="flex-col items-center">
          <View testID={`tab-status-${id}`} accessibilityLabel={`status-icon:${working() ? "working" : agent() ? tab()?.activity?.app : statusIcon()}`}>
            <If when={working()}><WorkingIndicator kind={props.indicator()} color={statusColor()} /></If>
            <If when={!working() && !!agent()}><AgentIcon app={tab()?.activity?.app ?? ""} /></If>
            <If when={!working() && !agent()}><Icon name={statusIcon()} size={13} color={statusColor()} /></If>
          </View>
          <If when={!props.expanded()}>{progressLine(true)}</If>
        </View>
        <If when={props.expanded()}>
          <View class="min-w-0 flex-1 flex-col justify-center">
            <Text class="min-w-0 text-(length:--text-body)"
              style={{ numberOfLines: 1, color: token(active() ? "tab-selected-foreground" : "sidebar-foreground") }}>{tab()?.title}</Text>
            {progressLine(false)}
          </View>
        </If>
      </Button>
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
    const anchor = signal<ViewRef | null>(null);
    let trigger: ViewRef | undefined;
    const close = (restoreFocus = false) => {
      props.groupEditor.set(null);
      if (restoreFocus) trigger?.focus();
    };
    const visibleGit = () => (store.gitViews()[id]?.open ?? []).filter(kind =>
      !store.group(id)?.collapsed || (!props.expanded() && store.workspaceKey() === id && store.activeGitView() === kind));
    const shownSplit = () => {
      const all = splitSessions(id);
      if (all.length < 2) return [];
      if (store.group(id)?.collapsed && props.expanded()) return [];
      if (!props.expanded()) return !store.canvasBlocked() && all.includes(store.activeId() ?? "") ? [store.activeId()!] : [];
      return all;
    };
    const visibleMembers = () => members(id).filter((tab) => !splitSessions(id).includes(tab.id) && (!store.group(id)?.collapsed || (!props.expanded() && !store.canvasBlocked() && tab.id === store.activeId())));
    const hiddenCount = () => members(id).filter((tab) => !visibleMembers().some((item) => item.id === tab.id) && !shownSplit().includes(tab.id)).length
      + (store.gitViews()[id]?.open.length ?? 0) - visibleGit().length;
    return <View class="relative mb-2"
        classList={{ "mx-1 py-0.5": props.expanded() }}>
        <If when={!props.expanded()}>
          <View class="absolute w-px rounded-sm" style={{ left: 2, top: 2, bottom: 2, backgroundColor: groupLine(store.group(id)?.color) }} />
        </If>
        <View ref={(ref) => { anchor.set(ref); register(`header:${id}`, ref ? { ref, kind: "header", id } : null); }}
          testID={store.workspaceKey() === id ? `group-workspace-${id}` : undefined}
          class="group relative flex-row items-center rounded-(--radius-control)"
          classList={{ "mx-2": !props.expanded(), "bg-sidebar-accent": drop()?.headerId === id || store.workspaceKey() === id }}
          style={{ marginVertical: props.expanded() ? 0 : 2 }}
          onContextMenu={() => props.groupEditor.set(id)}>
          <If when={drop()?.headerId === id}>
            <View testID={`drop-group-${id}`} class="absolute inset-0 rounded-(--radius-control) border border-primary" style={{ pointerEvents: "none" }} />
          </If>
          <If when={props.expanded()}>
            <Button testID={`group-toggle-${id}`}
              accessibilityLabel={`${store.group(id)?.collapsed ? "Expand" : "Collapse"} ${store.groupTitle(id)}`}
              onPress={() => toggleGroup(id)}
              class="h-(--height-tab) w-5 shrink-0 items-center justify-center rounded-(--radius-control) hover:bg-control-hover active:bg-control-pressed focus-visible:ring-2 ring-sidebar-ring">
              <Icon name={store.group(id)?.collapsed ? "chevron" : "down"} size={11} color={token("subtle-foreground")} />
            </Button>
          </If>
          <Button ref={(ref) => { trigger = ref; }} testID={`group-${id}`}
            accessibilityLabel={`Open ${store.groupTitle(id)}${store.workspaceKey() === id ? ", current" : ""}`}
            onPress={() => store.selectGroup(id)}
            class="h-(--height-tab) min-w-0 flex-1 flex-row items-center gap-1.5 rounded-(--radius-control) focus-visible:ring-2 ring-sidebar-ring"
            style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 2 : 0 }}
            tooltip={<Hint>{`${store.groupTitle(id)} · ${store.group(id)?.cwd ?? ""} · ${members(id).length} terminals`}</Hint>}>
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
            <View class="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100" style={{ opacity: props.groupEditor() === id ? 1 : undefined }}>
              <IconButton icon="more" label={`Options for ${store.groupTitle(id)}`} testID={`group-menu-${id}`} onPress={() => props.groupEditor.set(props.groupEditor() === id ? null : id)} />
            </View>
          </If>
          <If when={props.groupEditor() === id && anchor() !== null}>
            <GroupEditor id={id} anchor={anchor()!} workspace={store} close={close} deleteGroup={() => actions.closeGroup(id)} />
          </If>
        </View>
        <If when={shownSplit().length > 0 || visibleMembers().length > 0 || visibleGit().length > 0}>
          <View style={{ marginLeft: props.expanded() ? 10 : 0, paddingLeft: props.expanded() ? token("gap-group-guide") : 4, paddingRight: props.expanded() ? 0 : 4, borderLeftWidth: props.expanded() ? 1 : 0, borderColor: groupLine(store.group(id)?.color) }}>
            <Each of={visibleGit()}>{kind =>
              <View class="group relative my-0.5 h-(--height-tab) flex-row items-center rounded-(--radius-tab) hover:bg-tab-hover"
                classList={{ "mx-1": !props.expanded(), "bg-tab-selected hover:bg-tab-selected": store.workspaceKey() === id && store.activeGitView() === kind() }}>
                <Button testID={`git-tab-${id}-${kind()}`} role="tab" selected={store.workspaceKey() === id && store.activeGitView() === kind()} accessibilityLabel={`${kind() === "changes" ? "Changes" : "History"} in ${store.groupTitle(id)}`}
                  onPress={() => store.openGitView(kind(), id)}
                  tooltip={<Hint>{`${kind() === "changes" ? "Changes" : "History"} · ${store.groupTitle(id)}`}</Hint>}
                  class="h-full min-w-0 flex-1 flex-row items-center gap-2 rounded-(--radius-tab) focus-visible:ring-2 ring-sidebar-ring"
                  style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 7 : 0 }}>
                  <Icon name={kind() === "changes" ? "gitBranch" : "gitCommit"} size={13} color={groupColor(store.group(id)?.color)} />
                  <If when={props.expanded()}><Text class="min-w-0 flex-1 text-(length:--text-body) text-sidebar-foreground">{kind() === "changes" ? "Changes" : "History"}</Text></If>
                </Button>
                <If when={props.expanded()}><View class="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
                  style={{ opacity: store.workspaceKey() === id && store.activeGitView() === kind() ? 1 : undefined }}>
                  <IconButton icon="close" label={`Close ${kind() === "changes" ? "Changes" : "History"}`} testID={`close-git-${id}-${kind()}`} onPress={() => store.closeGitView(id, kind())} />
                </View></If>
              </View>
            }</Each>
            <If when={shownSplit().length > 0}><SplitCard groupId={id} /></If>
            <Each of={visibleMembers().map((t) => t.id)}>{(tabId) => <TabRow id={tabId()} />}</Each>
          </View>
        </If>
        <If when={!props.expanded() && hiddenCount() > 0}>
          <Button testID={`group-hidden-${id}`} accessibilityLabel={`Show ${hiddenCount()} hidden tabs in ${store.groupTitle(id)}`}
            onPress={() => store.patchGroup(id, { collapsed: false })}
            class="mx-2 my-0.5 h-(--height-tab) items-center justify-center rounded-(--radius-tab) focus-visible:ring-2 ring-sidebar-ring">
            <View class="h-(--height-tab) w-(--height-tab) items-center justify-center rounded-full"
              style={{ backgroundColor: groupTint(store.group(id)?.color) }}>
              <Text class="text-(length:--text-label)" style={{ color: groupColor(store.group(id)?.color) }}>+{hiddenCount()}</Text>
            </View>
          </Button>
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
    <If when={props.expanded()}>
      <View testID="sidebar-heading" class="h-(--height-toolbar) shrink-0 flex-row items-center px-3">
        <Type role="label">Sessions</Type>
      </View>
    </If>

    <Scroll class="min-h-0 flex-1" classList={{ "pt-1.5": props.expanded() }} scrollbar={false}>
      <View testID="ungrouped-section" class="mb-2 pb-2" classList={{ "border-b border-sidebar-border": props.expanded() }}>
      <View class="relative justify-center" classList={{ "h-(--height-toolbar) border-b border-sidebar-border": !props.expanded() }}>
      <If when={store.workspaceKey() === ""}><View testID="ungrouped-active" class="absolute left-1 w-0.5 rounded-full bg-primary" classList={{ "bottom-1 top-1": props.expanded(), "bottom-2.5 top-2.5": !props.expanded() }} style={{ pointerEvents: "none" }} /></If>
      <Button testID="workspace-ungrouped" accessibilityLabel="Open Ungrouped workspace" onPress={store.selectUngrouped}
        tooltip={<Hint>Ungrouped</Hint>}
        class="mx-1 h-(--height-tab) flex-row items-center gap-2 rounded-(--radius-control) hover:bg-sidebar-accent focus-visible:ring-2 ring-sidebar-ring"
        style={{ justifyContent: props.expanded() ? "flex-start" : "center", paddingHorizontal: props.expanded() ? 9 : 0 }}>
        <Icon name="ungrouped" size={13} color={token(store.workspaceKey() === "" ? "icon-active" : "icon")} />
        <If when={props.expanded()}><Type role="label" tone={store.workspaceKey() === "" ? "foreground" : "muted-foreground"} class="min-w-0 flex-1">UNGROUPED</Type></If>
      </Button>
      </View>
      <If when={splitSessions(null).length >= 2 && props.expanded()}><SplitCard groupId={null} /></If>
      <If when={splitSessions(null).length >= 2 && !props.expanded() && splitSessions(null).includes(store.activeId() ?? "")}>
        <SplitCard groupId={null} />
      </If>
      <Each of={store.tabs().filter((t) => t.groupId === null && t.pinned && !splitSessions(null).includes(t.id)).map((t) => t.id)}>{(id) => <TabRow id={id()} />}</Each>
      <View ref={(ref) => register("ungrouped", ref ? { ref, kind: "ungrouped", id: null } : null)}
        testID="ungrouped-drop" class="mx-2 h-1" />
      <Each of={members(null).filter((t) => !t.pinned && !splitSessions(null).includes(t.id)).map((t) => t.id)}>{(id) => <TabRow id={id()} />}</Each>
      <If when={appendLine(null, false)}>
        <View testID="drop-line" accessibilityLabel="Insert terminal" class="mx-3 h-0.5 bg-primary" />
      </If>
      </View>

      <Each of={store.groups().map((g) => g.id)}>{(id) => <GroupRow id={id()} />}</Each>

      <Button testID="new-terminal" accessibilityLabel="New terminal" onPress={actions.newTerminal}
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
