import { Overlay, Scroll, signal, token, View, type ViewRef } from "nsh";
import { Button } from "nsh/base";
import { DialogKeys, IconButton } from "../shared/controls";
import { Type } from "../shared/type";
import { NotificationRow } from "./NotificationRow";
import { Icon } from "../shared/icons";
import type { Notice } from "termlevel/native";

export function NotificationPanel(props: {
  anchor: ViewRef;
  windowHeight: number;
  windowWidth: number;
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
    style={{ maxWidth: Math.max(0, props.windowWidth - 16), boxShadow: [{ x: 0, y: 8, blur: 24, color: token("panel-shadow") }] }}>
    <DialogKeys dismiss={props.close} />
    <View class="h-(--height-toolbar) flex-row items-center gap-2 border-b border-border px-3">
      <Icon name="bell" size={16} /><Type role="body" class="flex-1 font-semibold">Notifications</Type>
      <Button testID="notifications-clear" disabled={!props.entries.length} onPress={props.clear} class="rounded-(--radius-control) px-2 py-1 hover:bg-control-hover">
        <Type role="label">Clear all</Type>
      </Button>
      <IconButton icon="close" label="Close notifications" onPress={props.close} />
    </View>
    <If when={!props.entries.length}>
      <View testID="notifications-empty" class="items-center gap-2 px-4 py-6">
        <View class="h-10 w-10 items-center justify-center rounded-full bg-muted"><Icon name="bell" size={20} /></View>
        <Type role="body" class="font-semibold">No notifications</Type>
        <Type role="caption">Terminal messages and app warnings will appear here.</Type>
      </View>
    </If>
    <If when={props.entries.length > 0}>
      <Scroll testID="notifications-history" scrollbar={true} style={{ height: Math.min(contentHeight(), token("height-notifications-max"), Math.max(64, props.windowHeight - token("height-titlebar") - token("height-toolbar") - token("height-notifications-footer") - 2 * token("gap-pane") - 2)) }}>
        <View>
          <Each of={props.entries} key={entry => entry.id}>{(entry) =>
            <View testID={`notification-${entry().id}`} class="border-b border-border"
              onLayout={({ bounds }) => {
                const height = bounds.size.height;
                if (height > 0 && heights()[entry().id] !== height) {
                  const current = new Set(props.entries.map(item => item.id));
                  heights.set({ ...Object.fromEntries(Object.entries(heights()).filter(([id]) => current.has(id))), [entry().id]: height });
                }
              }}>
              <NotificationRow entry={entry()} sourceTitle={entry().sessionId ? props.sessionTitle(entry().sessionId!) : undefined}
                dismiss={() => props.dismiss(entry().id)} select={() => { if (entry().sessionId) props.select(entry().sessionId!); }} />
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
