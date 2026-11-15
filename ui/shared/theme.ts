import { token } from "nsh";

/** Group identity is data; its rendered colors belong to theme.json. */
export const groupPalette = [
  { id: "blue", label: "Blue" },
  { id: "purple", label: "Purple" },
  { id: "cyan", label: "Cyan" },
  { id: "orange", label: "Orange" },
  { id: "yellow", label: "Yellow" },
  { id: "pink", label: "Pink" },
  { id: "green", label: "Green" },
  { id: "gray", label: "Gray" },
  { id: "red", label: "Red" },
] as const;

export type GroupColor = typeof groupPalette[number]["id"];

/** The palette ids as plain strings, for validating persisted group state. */
export const groupColorIds: readonly GroupColor[] = groupPalette.map((item) => item.id);

export const groupColor = (color: GroupColor = "purple") => token(`group-${color}`);
export const groupTint = (color: GroupColor = "purple") => token(`group-${color}-muted`);
export const groupLine = (color: GroupColor = "purple") => token(`group-${color}-line`);

/** Convert a theme color to an ANSI truecolor run for the preview transport. */
export function ansiColor(color: string) {
  const rgb = color.replace(/^#/, "").slice(0, 6);
  return `\x1b[38;2;${parseInt(rgb.slice(0, 2), 16)};${parseInt(rgb.slice(2, 4), 16)};${parseInt(rgb.slice(4, 6), 16)}m`;
}
