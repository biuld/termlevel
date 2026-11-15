import { Canvas, Path2D, token } from "nsh";

const paths = {
  splitRight: "M4 4H20V20H4Z M12 4V20 M15 12H19 M17 10L19 12L17 14",
  splitDown: "M4 4H20V20H4Z M4 12H20 M12 15V19 M10 17L12 19L14 17",
  maximize: "M9 4H4V9 M15 4H20V9 M4 15V20H9 M20 15V20H15",
  restore: "M4 9H9V4 M15 4V9H20 M4 15H9V20 M15 20V15H20",
  terminal: "M4 5L10 11L4 17 M12 17H20",
  plus: "M12 5V19 M5 12H19",
  close: "M6 6L18 18 M18 6L6 18",
  chevron: "M9 5L16 12L9 19",
  down: "M5 9L12 16L19 9",
  sidebar: "M4 4H20V20H4Z M10 4V20",
  folder: "M3 6H10L12 8H21V19H3Z",
  more: "M5 11H6V12H5Z M11 11H12V12H11Z M17 11H18V12H17Z",
  settings: "M9 3H15L16 6L19 7L22 12L19 17L16 18L15 21H9L8 18L5 17L2 12L5 7L8 6Z M12 8A4 4 0 1 0 12 16A4 4 0 1 0 12 8",
  pin: "M8 3H16L15 9L18 12V14H6V12L9 9Z M12 14V21",
  search: "M10 3A7 7 0 1 0 10 17A7 7 0 1 0 10 3 M15 15L21 21",
  arrow: "M5 12H19 M13 6L19 12L13 18",
  bell: "M6 10A6 6 0 0 1 18 10V15L20 18H4L6 15Z M10 21H14",
  book: "M4 4H10L12 6L14 4H20V19H14L12 21L10 19H4Z M12 6V21",
  busy: "M4.5 12H8 M10.25 12H13.75 M16 12H19.5",
  check: "M5 12L10 17L19 7",
  alert: "M12 3A9 9 0 1 0 12 21A9 9 0 1 0 12 3 M12 8V13 M12 16.5V17",
  xCircle: "M12 3A9 9 0 1 0 12 21A9 9 0 1 0 12 3 M9 9L15 15 M15 9L9 15",
} as const;

export type IconName = keyof typeof paths;

export function Icon(props: { name: IconName; size?: number; color?: string }) {
  return <Canvas
    style={{ width: props.size ?? 18, height: props.size ?? 18, pointerEvents: "none" }}
    draw={(cx, { width }) => {
      cx.scale(width / 24, width / 24);
      cx.strokeStyle = props.color ?? token("icon");
      cx.lineWidth = 1.65;
      cx.lineCap = "round";
      cx.lineJoin = "round";
      cx.stroke(new Path2D(paths[props.name]));
    }}
  />;
}
