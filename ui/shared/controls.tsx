import { Text, token, View, type JSX } from "nsh";
import { Button, Dialog } from "nsh/base";
import { Icon, type IconName } from "./icons";
import { Type } from "./type";
import { DialogKeys as NativeDialogKeys } from "termlevel/native";

/** A compact, opaque hint with the same surface as other popovers. */
export function Hint(props: { children: string; lines?: number }) {
  return <Text
    class="rounded-(--radius-overlay) border border-border bg-popover px-2 py-1 text-(length:--text-caption) text-popover-foreground"
    style={{ numberOfLines: props.lines, boxShadow: [{ x: 0, y: 2, blur: 8, color: token("panel-shadow") }] }}
  >{props.children}</Text>;
}

/** Bind inside the dialog's mounted scope, after terminal input handlers. */
export function DialogKeys(props: { dismiss: () => void }) {
  return <NativeDialogKeys onDismiss={() => props.dismiss()} />;
}

/** Inset field on the window floor. Same surface as a sidebar pane. */
export const fieldClass = "h-(--height-input) rounded-(--radius-control) border border-sidebar-border bg-sidebar px-3 text-(length:--text-body) text-foreground";

/** Centered modal on NSH `Dialog`. The runtime supplies the layer; this draws the window floor. */
export function AppDialog(props: {
  testID?: string;
  open: boolean;
  label: string;
  title?: string;
  description?: string;
  role?: "dialog" | "alertdialog";
  /** Width class, for example `w-[320px]`. */
  frame: string;
  dismiss: () => void;
  closeLabel?: string;
  /** When false, children sit edge to edge under the title bar. */
  padded?: boolean;
  children?: JSX.Element;
}) {
  const alert = props.role === "alertdialog" || props.description != null;
  return <Dialog testID={props.testID} open={props.open} animation="fade" role={props.role ?? "dialog"}
    accessibilityLabel={props.label} accessibilityDescription={props.description}
    onOpenChange={({ open }) => { if (!open) props.dismiss(); }}
    class={`m-auto overflow-hidden rounded-(--radius-panel) border border-sidebar-border bg-chrome ${props.frame}`}
    backdropClass="bg-scrim"
    style={{ boxShadow: [{ x: 0, y: 12, blur: 32, color: token("panel-shadow") }] }}>
    <DialogKeys dismiss={props.dismiss} />
    <If when={!alert && props.title != null}>
      <View class="h-(--height-toolbar) flex-row items-center gap-2 px-2">
        <Type role="body" class="min-w-0 flex-1 px-1" lines={1}>{props.title}</Type>
        <If when={props.closeLabel != null}>
          <IconButton icon="close" label={props.closeLabel ?? ""} onPress={props.dismiss} />
        </If>
      </View>
    </If>
    <View class={alert ? "gap-3 px-4 py-4" : props.padded === false ? "" : "gap-3 px-3 pb-3"}>
      <If when={alert && props.title != null}>
        <View class="gap-1">
          <Type role="title">{props.title}</Type>
          <If when={props.description != null}>
            <Type role="body" tone="muted-foreground">{props.description}</Type>
          </If>
        </View>
      </If>
      {props.children}
    </View>
  </Dialog>;
}

/** Right-aligned actions on the window floor. */
export function DialogActions(props: { children: JSX.Element }) {
  return <View class="flex-row justify-end gap-2">{props.children}</View>;
}

export function IconButton(props: {
  icon: IconName;
  label: string;
  onPress: () => void;
  testID?: string;
  color?: string;
  selected?: boolean;
  disabled?: boolean;
  /** Circle, so a hover fill matches a round control. */
  circle?: boolean;
}) {
  return <Button
    testID={props.testID}
    accessibilityLabel={props.label}
    onPress={props.onPress}
    disabled={props.disabled}
    tooltip={<Hint>{props.label}</Hint>}
    class={`h-(--height-control) w-(--height-control) shrink-0 items-center justify-center hover:bg-control-hover active:bg-control-pressed focus-visible:ring-2 ring-ring ${props.circle ? "rounded-full" : "rounded-(--radius-control)"}`}
    classList={{ "bg-control-selected": props.selected, "opacity-40": props.disabled }}
  >
    <Icon name={props.icon} color={props.color ?? token(props.selected ? "control-selected-foreground" : "icon")} size={token("icon-size")} />
  </Button>;
}

export function Action(props: { children: JSX.Element; onPress: () => void; primary?: boolean; destructive?: boolean; testID?: string }) {
  return <Button
    testID={props.testID}
    onPress={props.onPress}
    class="h-(--height-action) items-center justify-center rounded-(--radius-control) px-3 focus-visible:ring-2 ring-ring"
    classList={{
      "bg-tab-selected": props.primary && !props.destructive,
      "hover:bg-control-hover active:bg-control-pressed": !props.primary && !props.destructive,
      "bg-destructive": props.destructive,
    }}
  >
    <Text class="text-(length:--text-body) font-medium" style={{ color: token(props.destructive ? "destructive-foreground" : props.primary ? "tab-selected-foreground" : "foreground") }}>{props.children}</Text>
  </Button>;
}
