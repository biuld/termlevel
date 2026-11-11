import { Text, token, type JSX } from "nsh";
import { Button } from "nsh/base";
import { Icon, type IconName } from "./icons";
import { DialogKeys as NativeDialogKeys } from "termlevel/native";

/** Bind inside the dialog's mounted scope, after terminal input handlers. */
export function DialogKeys(props: { dismiss: () => void }) {
  return <NativeDialogKeys onDismiss={() => props.dismiss()} />;
}

export function IconButton(props: {
  icon: IconName;
  label: string;
  onPress: () => void;
  testID?: string;
  color?: string;
  selected?: boolean;
}) {
  return <Button
    testID={props.testID}
    accessibilityLabel={props.label}
    onPress={props.onPress}
    tooltip={<Text class="rounded-(--radius-control) border border-border bg-popover px-2 py-1 text-(length:--text-caption) text-popover-foreground">{props.label}</Text>}
    class="h-(--height-control) w-(--height-control) shrink-0 items-center justify-center rounded-(--radius-control) hover:bg-control-hover active:bg-control-pressed focus-visible:ring-2 ring-ring"
    classList={{ "bg-control-selected": props.selected }}
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
