import { signal, token, View } from "nsh";
import { Button } from "nsh/base";
import type { Notice } from "termlevel/native";
import { Icon, type IconName } from "../shared/icons";
import { IconButton } from "../shared/controls";
import { Type } from "../shared/type";

/** Bounded text previews never discard the full history payload. */
function excerpt(text: string, limit: number) {
  const firstLines = text.split("\n").slice(0, 3).join("\n");
  const value = Array.from(firstLines).slice(0, limit).join("");
  return value === text ? value : `${value.trimEnd()}…`;
}

export function NotificationRow(props: {
  entry: Notice;
  sourceTitle?: string;
  dismiss: () => void;
  select: () => void;
}) {
  const expanded = signal(false);
  const program = () => props.entry.program;
  const category = () => {
    if (props.entry.level === "error") return program() ? "Task failed" : "Error";
    if (program()) {
      switch (program()?.kind) {
        case "permission": return "Needs approval";
        case "question": return "Needs answer";
        case "auth": return "Sign in required";
        default: return "Needs input";
      }
    }
    return props.entry.level === "warning" ? "Warning" : "Notification";
  };
  const icon = (): IconName => {
    if (props.entry.level === "error") return "xCircle";
    if (program()) {
      switch (program()?.kind) {
        case "permission": return "shield";
        case "question": return "question";
        case "auth": return "lock";
        default: return "alert";
      }
    }
    return props.entry.level === "warning" ? "alert" : "info";
  };
  const tone = () => props.entry.level === "error" ? "destructive"
    : props.entry.level === "warning" ? "notification-warning" : "notification-info";
  const tint = () => props.entry.level === "error" ? "bg-notification-error-muted"
    : props.entry.level === "warning" ? "bg-notification-warning-muted" : "bg-notification-info-muted";
  // Program-supplied content leads; the generated event heading is metadata.
  // For ordinary notices too, the actual message leads and the event title
  // stays as context. The host's empty-title placeholder adds no information.
  const headline = () => props.entry.source === "app" && props.entry.level === "info" ? props.entry.title
    : program()?.message || program()?.title || props.entry.body || props.entry.title;
  const body = () => props.entry.source === "app" && props.entry.level === "info" ? props.entry.body
    : program() ? (program()?.message && program()?.title !== headline() ? program()?.title ?? "" : "")
    : props.entry.body && props.entry.title !== "Terminal notification" && props.entry.title !== headline() ? props.entry.title : "";
  const canExpand = () => excerpt(headline(), 100) !== headline() || excerpt(body(), 160) !== body();
  const origin = () => [props.entry.source === "app" ? "Termlevel" : "Terminal", program()?.app].filter(Boolean).join(" · ");
  return <View class="flex-row items-start gap-3 px-3 py-3">
    <View testID={`notification-icon-${props.entry.id}`} accessibilityLabel={category()} role="image"
      class={`mt-0.5 shrink-0 items-center justify-center rounded-(--radius-dialog) ${tint()}`}
      style={{ width: token("size-notification-icon"), height: token("size-notification-icon") }}>
      <Icon name={icon()} size={18} color={token(tone())} />
    </View>
    <View class="min-w-0 flex-1 gap-1">
      <View class="flex-row items-center gap-2">
        <Type role="caption" class="min-w-0 flex-1" lines={1}>{origin()} · {category()}</Type>
        <If when={props.entry.count > 1}>
          <View class="rounded-full bg-muted px-1.5"><Type role="caption">×{props.entry.count}</Type></View>
        </If>
        <Type role="caption" tone="subtle-foreground">{new Date(props.entry.createdAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</Type>
      </View>
      <Type testID={`notification-content-${props.entry.id}`} role="body" class={expanded() && excerpt(headline(), 100) !== headline() ? "" : "font-semibold"} tone="popover-foreground">
        {expanded() ? headline() : excerpt(headline(), 100)}
      </Type>
      <If when={!!body()}><Type role="body" tone="popover-foreground">{expanded() ? body() : excerpt(body(), 160)}</Type></If>
      <View class="mt-1 flex-row items-center gap-1">
        <If when={props.entry.sessionId != null}>
          <Button testID={`source-${props.entry.id}`} accessibilityLabel={`Open source of ${props.entry.title}`}
            disabled={!props.sourceTitle} onPress={props.select}
            class="min-w-0 flex-row items-center gap-1 rounded-(--radius-control) px-1.5 py-1 hover:bg-control-hover focus-visible:ring-2 ring-ring">
            <Icon name="terminal" size={12} />
            <Type role="caption" lines={1}>{props.sourceTitle ?? "Terminal closed"}</Type>
            <If when={!!props.sourceTitle}><Icon name="arrow" size={12} /></If>
          </Button>
        </If>
        <If when={canExpand()}>
          <Button testID={`expand-${props.entry.id}`} accessibilityLabel={`${expanded() ? "Collapse" : "Expand"} ${props.entry.title} notification`}
            onPress={() => expanded.set(!expanded())} class="shrink-0 rounded-(--radius-control) px-1.5 py-1 hover:bg-control-hover focus-visible:ring-2 ring-ring">
            <Type role="caption">{expanded() ? "Show less" : "Show more"}</Type>
          </Button>
        </If>
        <View class="flex-1" />
        <IconButton icon="close" label={`Dismiss ${props.entry.title} notification`} testID={`dismiss-${props.entry.id}`} onPress={props.dismiss} />
      </View>
    </View>
  </View>;
}
