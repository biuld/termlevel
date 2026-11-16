import { Text, token, type JSX } from "nsh";
import { interfaceFont, monoFont } from "./fonts";

/** Roles for the interface. `mono` is the shortcut size; paths use `body` with `family="mono"`. */
export type TypeRole = "heading" | "title" | "body" | "label" | "caption" | "mono";

const size = {
  heading: "text-heading",
  title: "text-title",
  body: "text-body",
  label: "text-label",
  caption: "text-caption",
  mono: "text-caption",
} as const;

const leading = {
  heading: "leading-heading",
  title: "leading-title",
  body: "leading-body",
  label: "leading-label",
  caption: "leading-caption",
  mono: "leading-caption",
} as const;

const weight = {
  heading: "font-medium",
  title: "font-semibold",
  body: "",
  label: "font-medium",
  caption: "",
  mono: "",
} as const;

const color = {
  heading: "foreground",
  title: "foreground",
  body: "foreground",
  label: "muted-foreground",
  caption: "muted-foreground",
  mono: "subtle-foreground",
} as const;

/** One text style. Pass `class` for layout only. `tone` replaces the role's color. */
export function Type(props: {
  role: TypeRole;
  class?: string;
  /** `mono` uses the terminal family. Size still comes from `role`. */
  family?: "ui" | "mono";
  tone?: "foreground" | "muted-foreground" | "subtle-foreground" | "popover-foreground" | "destructive";
  testID?: string;
  label?: string;
  tooltip?: JSX.Element;
  children?: JSX.Element;
  lines?: number;
}) {
  return <Text
    testID={props.testID}
    accessibilityLabel={props.label}
    tooltip={props.tooltip}
    class={`${weight[props.role]} ${props.class ?? ""}`}
    style={{
      fontFamily: props.family === "mono" || props.role === "mono" ? monoFont() : interfaceFont(),
      fontSize: token(size[props.role]),
      lineHeight: token(leading[props.role]),
      color: token(props.tone ?? color[props.role]),
      numberOfLines: props.lines,
    }}
  >{props.children}</Text>;
}
