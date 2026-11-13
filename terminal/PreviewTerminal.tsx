import { cleanup, effect, token, untracked, View } from "nsh";
import { monoFont } from "../ui/fonts";
import { Terminal, type TerminalRef } from "nsh/base";
import type { Workspace } from "../ui/workspace";
import { PreviewSession } from "./preview-session";
import { ansiColor } from "../ui/theme";

export function PreviewTerminal(props: { id: string; workspace: Workspace }) {
  const store = props.workspace;
  const id = props.id;
  const cwd = store.tab(id)?.cwd ?? "~";
  let terminal: TerminalRef | undefined;
  let started = false;
  let disposed = false;
  const preview = new PreviewSession((text) => terminal?.write(text), cwd, () => ({
    brand: ansiColor(token("terminal-brand")),
    prompt: ansiColor(token("terminal-prompt")),
    hint: ansiColor(token("terminal-hint")),
  }));
  const focusIfActive = () => {
    if (!disposed && store.activeId() === id) terminal?.focus();
  };

  cleanup(() => { disposed = true; });
  effect(() => {
    const active = store.activeId() === id;
    if (active) untracked(focusIfActive);
  });

  return <View
    testID={`surface-${id}`}
    accessibilityElementsHidden={store.activeId() !== id}
    style={{
      position: "absolute", top: 0, right: 0, bottom: 0, left: 0,
      opacity: store.activeId() === id ? 1 : 0,
      pointerEvents: store.activeId() === id ? "auto" : "none",
    }}
  >
    <Terminal
      testID={`terminal-${id}`}
      accessibilityLabel={`Terminal ${id}`}
      ref={(ref) => { terminal = ref; }}
      fontFamily={monoFont()}
      fontSize={store.fontSize()}
      lineHeight={store.fontSize() + token("line-gap-terminal")}
      scrollback={3000}
      cursorClass="bg-terminal-cursor/55"
      class="flex-1 bg-terminal-background text-terminal-foreground px-(--padding-terminal) pb-(--padding-terminal)"
      onData={({ data }) => preview.input(data)}
      onResize={() => {
        if (started) return;
        started = true;
        preview.start();
        focusIfActive();
      }}
    />
  </View>;
}
