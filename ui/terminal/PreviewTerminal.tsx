import { cleanup, effect, signal, Text, token, untracked, View, type Signal } from "nsh";
import { monoFont } from "../shared/fonts";
import { Terminal, type TerminalRef } from "nsh/base";
import type { Workspace } from "../workspace/workspace";
import { FindBar, countMatches } from "./FindBar";
import { PreviewSession } from "./preview-session";
import { ansiColor } from "../shared/theme";

export function PreviewTerminal(props: {
  id: string;
  workspace: Workspace;
  finding: Signal<boolean>;
  findQuery: Signal<string>;
}) {
  const store = props.workspace;
  const id = props.id;
  const cwd = store.tab(id)?.cwd ?? "~";
  let terminal: TerminalRef | undefined;
  let paintedScrollback = -1;
  let disposed = false;
  const preview = new PreviewSession((text) => terminal?.write(text), cwd, () => ({
    brand: ansiColor(token("terminal-brand")),
    prompt: ansiColor(token("terminal-prompt")),
    hint: ansiColor(token("terminal-hint")),
  }));
  const matchCount = signal(0);
  const matchIndex = signal(0);
  const focusIfActive = () => {
    if (!disposed && store.activeId() === id && !store.inputBlocked() && !props.finding()) terminal?.focus();
  };
  const showingFind = () => props.finding() && store.activeId() === id;
  effect(() => {
    const query = props.findQuery();
    const open = showingFind();
    if (!open) {
      matchCount.set(0);
      matchIndex.set(0);
      return;
    }
    const current = terminal;
    void (async () => {
      const found = countMatches(await current?.text() ?? "", query);
      if (props.findQuery() !== query || !(props.finding() && store.activeId() === id)) return;
      matchCount.set(found);
      matchIndex.set(found ? 1 : 0);
    })();
  });
  const moveMatch = (delta: number) => {
    const count = matchCount();
    if (!count) return;
    matchIndex.set(((matchIndex() - 1 + delta) % count + count) % count + 1);
  };

  cleanup(() => { disposed = true; });
  effect(() => {
    const active = store.activeId() === id && !store.inputBlocked();
    if (active) untracked(focusIfActive);
  });

  return <View
    testID={`surface-${id}`}
    class="flex-col"
    accessibilityElementsHidden={!store.frameForSession(id)}
    style={{
      position: "absolute", top: 0, right: 0, bottom: 0, left: 0,
      opacity: store.frameForSession(id) ? 1 : 0,
      pointerEvents: store.frameForSession(id) ? "auto" : "none",
    }}
  >
    <If when={showingFind()}>
      <FindBar query={props.findQuery} count={matchCount} index={matchIndex}
        onQuery={props.findQuery.set} onMove={moveMatch} onClose={() => props.finding.set(false)} />
    </If>
    <View class="min-h-0 flex-1">
      <Each of={[store.scrollback()]} key={(limit) => limit}>{(limit) =>
        <Terminal
          testID={`terminal-${id}`}
          accessibilityLabel={`Terminal ${id}`}
          ref={(ref) => { terminal = ref; }}
          fontFamily={monoFont()}
          fontSize={store.fontSize()}
          lineHeight={store.fontSize() + token("line-gap-terminal")}
          scrollback={limit()}
          cursorClass="bg-terminal-cursor/55"
          class="flex-1 bg-terminal-background text-terminal-foreground px-(--padding-terminal) pb-(--padding-terminal)"
          onData={({ data }) => {
            if (store.activeId() !== id || store.inputBlocked()) return;
            const code = preview.input(data);
            if (code === 0) store.closeCleanExit(id);
            else if (typeof code === "number") store.patchTab(id, { state: "exited", exitCode: code, error: null });
          }}
          onResize={() => {
            const next = limit();
            if (paintedScrollback === next) return;
            paintedScrollback = next;
            preview.start();
            focusIfActive();
          }}
        />
      }</Each>
    </View>
    <If when={store.tab(id)?.state === "exited" || store.tab(id)?.state === "error"}>
      <View class="shrink-0 border-t border-border bg-popover px-3 py-2">
        <Text testID={`session-status-${id}`} class="text-(length:--text-label) text-muted-foreground">
          {store.tab(id)?.error ?? `Process exited with code ${store.tab(id)?.exitCode ?? "unknown"}. Open a new terminal to continue.`}
        </Text>
      </View>
    </If>
  </View>;
}
