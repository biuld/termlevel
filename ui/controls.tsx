import { Text, token, View, type JSX } from "nsh";
import { Button, Dialog } from "nsh/base";
import { Icon, type IconName } from "./icons";
import { Type } from "./type";
import { DialogKeys as NativeDialogKeys } from "termlevel/native";

/** A compact, opaque hint with the same surface as other popovers. */
export function Hint(props: { children: string }) {
  return <Text
    class="rounded-(--radius-overlay) border border-border bg-popover px-2 py-1 text-(length:--text-caption) text-popover-foreground"
    style={{ boxShadow: [{ x: 0, y: 2, blur: 8, color: token("panel-shadow") }] }}
  >{props.children}</Text>;
}

/** Bind inside the dialog's mounted scope, after terminal input handlers. */
export function DialogKeys(props: { dismiss: () => void }) {
  return <NativeDialogKeys onDismiss={() => props.dismiss()} />;
}

/** Centered modal on NSH `Dialog`. The runtime supplies the layer; this draws the solid popup. */
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
  children?: JSX.Element;
}) {
  return <Dialog testID={props.testID} open={props.open} animation="fade" role={props.role ?? "dialog"}
    accessibilityLabel={props.label} accessibilityDescription={props.description}
    onOpenChange={({ open }) => { if (!open) props.dismiss(); }}
    class={`m-auto gap-3 rounded-(--radius-dialog) border border-border bg-popover p-4 ${props.frame}`}
    backdropClass="bg-scrim"
    style={{ boxShadow: [{ x: 0, y: 16, blur: 40, color: token("panel-shadow") }] }}>
    <DialogKeys dismiss={props.dismiss} />
    <If when={props.title != null}>
      <View class="flex-row items-start gap-2">
        <View class="min-w-0 flex-1 gap-1">
          <Type role="title">{props.title}</Type>
          <If when={props.description != null}>
            <Type role="body" tone="muted-foreground">{props.description}</Type>
          </If>
        </View>
        <If when={props.closeLabel != null}>
          <IconButton icon="close" label={props.closeLabel ?? ""} onPress={props.dismiss} />
        </If>
      </View>
    </If>
    {props.children}
  </Dialog>;
}

/** Right-aligned actions under a hairline. */
export function DialogActions(props: { children: JSX.Element }) {
  return <View class="flex-row justify-end gap-2 border-t border-border pt-3">{props.children}</View>;
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
    class="h-(--height-action) items-center justify-center rounded-(--radius-control) px-3 hover:opacity-90 active:opacity-80 focus-visible:ring-2 ring-ring"
    classList={{ "bg-primary": props.primary && !props.destructive, "bg-secondary": !props.primary && !props.destructive, "bg-destructive": props.destructive }}
  >
    <Text class="text-(length:--text-body) font-medium" style={{ color: token(props.destructive ? "destructive-foreground" : props.primary ? "primary-foreground" : "secondary-foreground") }}>{props.children}</Text>
  </Button>;
}
