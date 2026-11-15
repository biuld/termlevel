import { Overlay, Scroll, signal, token, View, type ViewRef } from "nsh";
import { Button } from "nsh/base";
import { DialogKeys, IconButton } from "../shared/controls";
import { Type } from "../shared/type";
import type { Notice } from "termlevel/native";

export function NotificationPanel(props: {
  anchor: ViewRef;
  windowHeight: number;
  entries: Notice[];
  close: () => void;
  clear: () => void;
  dismiss: (id: string) => void;
  sessionTitle: (id: string) => string | undefined;
  select: (id: string) => void;
}) {
  const heights = signal<Record<string, number>>({});
  const contentHeight = () => props.entries.slice(0, 3).reduce((sum, entry) => sum + (heights()[entry.id] ?? token("height-notification-estimate")), 0);
  return <Overlay testID="notification-panel" role="dialog" accessibilityLabel="Notifications"
    anchor={props.anchor} placement="bottom" align="end" offset={8} margin={8}
    onPressOutside={props.close} onCancel={props.close}
    class="w-(--width-notifications) overflow-hidden rounded-(--radius-overlay) border border-border bg-popover"
    style={{ boxShadow: [{ x: 0, y: 8, blur: 24, color: token("panel-shadow") }] }}>
    <DialogKeys dismiss={props.close} />
    <View class="h-(--height-toolbar) flex-row items-center gap-2 border-b border-border px-3">
      <Type role="body" class="flex-1">Notifications</Type>
      <Button testID="notifications-clear" disabled={!props.entries.length} onPress={props.clear} class="rounded-(--radius-control) px-2 py-1 hover:bg-control-hover">
        <Type role="label">Clear all</Type>
      </Button>
      <IconButton icon="close" label="Close notifications" onPress={props.close} />
    </View>
    <If when={!props.entries.length}>
      <View testID="notifications-empty" class="gap-1 p-4">
        <Type role="body">No notifications</Type>
        <Type role="caption">Terminal messages and app warnings will appear here.</Type>
      </View>
    </If>
    <If when={props.entries.length > 0}>
      <Scroll testID="notifications-history" scrollbar={true} style={{ height: Math.min(contentHeight(), token("height-notifications-max"), Math.max(64, props.windowHeight - token("height-titlebar") - token("height-toolbar") - token("height-notifications-footer") - 2 * token("gap-pane") - 2)) }}>
        <View>
          <Each of={props.entries} key={entry => entry.id}>{(entry) =>
            <View testID={`notification-${entry().id}`} class="gap-1 border-b border-border px-3 py-2"
              onLayout={({ bounds }) => {
                const height = bounds.size.height;
                if (height > 0 && heights()[entry().id] !== height) {
                  const current = new Set(props.entries.map(item => item.id));
                  heights.set({ ...Object.fromEntries(Object.entries(heights()).filter(([id]) => current.has(id))), [entry().id]: height });
                }
              }}>
              <View class="flex-row items-center gap-2">
                <Type role="body" class="min-w-0 flex-1" lines={2}>{entry().title}</Type>
                <If when={entry().count > 1}><Type role="caption">×{entry().count}</Type></If>
                <IconButton icon="close" label={`Dismiss ${entry().title} notification`} testID={`dismiss-${entry().id}`} onPress={() => props.dismiss(entry().id)} />
              </View>
              <If when={!!entry().body}><Type role="body">{entry().body}</Type></If>
              <View class="flex-row items-center gap-2">
                <Type role="caption" class="flex-1">{entry().source === "app" ? `App · ${entry().level}` : "Terminal"} · {new Date(entry().createdAt).toLocaleTimeString()}</Type>
                <If when={entry().sessionId != null}>
                  <Button testID={`source-${entry().id}`} accessibilityLabel={`Open source of ${entry().title}`} disabled={!props.sessionTitle(entry().sessionId!)}
                    onPress={() => props.select(entry().sessionId!)} class="rounded-(--radius-control) px-2 py-1 hover:bg-control-hover">
                    <Type role="caption">{props.sessionTitle(entry().sessionId!) ?? "Terminal closed"}</Type>
                  </Button>
                </If>
              </View>
            </View>
          }</Each>
        </View>
      </Scroll>
      <View class="h-(--height-notifications-footer) justify-center border-t border-border px-3">
        <Type role="caption">{props.entries.length} / 100 kept this launch{props.entries.length > 3 ? " · Scroll for more" : ""}</Type>
      </View>
    </If>
  </Overlay>;
}
