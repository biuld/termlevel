import { setTheme, signal } from "nsh";
import theme from "../../theme.json";

/** Families NSH can resolve on macOS. `system-ui` is the platform font; SF Pro Text is not an installed family. */
export const interfaceFonts = [
  { id: "system-ui", label: "System" },
  { id: "Helvetica Neue", label: "Helvetica Neue" },
  { id: "Avenir Next", label: "Avenir Next" },
  { id: "Gill Sans", label: "Gill Sans" },
  { id: "Futura", label: "Futura" },
  { id: "Georgia", label: "Georgia" },
] as const;

/** Monospace families for the terminal and the toolbar path. */
export const monoFonts = [
  { id: "JetBrains Mono, Sarasa Term SC, ui-monospace", label: "JetBrains Mono" },
  { id: "Menlo", label: "Menlo" },
  { id: "Monaco", label: "Monaco" },
  { id: "Courier New", label: "Courier New" },
  { id: "Andale Mono", label: "Andale Mono" },
  { id: "ui-monospace", label: "System Mono" },
] as const;

export const interfaceFont = signal<string>(interfaceFonts[0].id);
export const monoFont = signal<string>(theme.cssVars.theme["font-mono"]);

/** Point the theme tokens at the chosen families so `font-sans` and `font-mono` follow. */
export function applyFonts() {
  setTheme({
    cssVars: {
      theme: { ...theme.cssVars.theme, "font-sans": interfaceFont(), "font-mono": monoFont() },
      light: theme.cssVars.light,
      dark: theme.cssVars.dark,
    },
  });
}
