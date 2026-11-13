import { effect, signal, Text, token, View, type Signal } from "nsh";
import { SessionTerminal, type SessionTerminalRef } from "termlevel/native";
import { monoFont } from "../ui/fonts";
import type { Workspace } from "../ui/workspace";
import { FindBar } from "./FindBar";
import { PreviewTerminal } from "./PreviewTerminal";

export function TerminalSurface(props: {
  id: string;
  workspace: Workspace;
  preview: boolean;
  finding: Signal<boolean>;
  findQuery: Signal<string>;
}) {
  if (props.preview) return <PreviewTerminal id={props.id} workspace={props.workspace} finding={props.finding} findQuery={props.findQuery} />;
  const store = props.workspace;
  const id = props.id;
  const tab = () => store.tab(id);
  let session: SessionTerminalRef | undefined;
  const matchCount = signal(0);
  const matchIndex = signal(0);
  const showingFind = () => props.finding() && store.activeId() === id;
  effect(() => {
    const query = props.findQuery();
    const open = showingFind();
    const current = session;
    if (!open) {
      current?.clearSearch();
      matchCount.set(0);
      matchIndex.set(0);
      return;
    }
    void (async () => {
      const found = await current?.search(query) ?? 0;
      if (props.findQuery() !== query || !(props.finding() && store.activeId() === id)) return;
      matchCount.set(found);
      matchIndex.set(found ? 1 : 0);
    })();
  });
  const moveMatch = (delta: number) => {
    const current = session;
    void (async () => {
      const next = await current?.searchMove(delta) ?? 0;
      if (next) matchIndex.set(next);
    })();
  };
  return <View testID={`surface-${id}`} class="flex-col"
    accessibilityElementsHidden={!store.frameForSession(id)}
    style={{
      position: "absolute", top: 0, right: 0, bottom: 0, left: 0,
      opacity: store.frameForSession(id) ? 1 : 0,
      pointerEvents: store.frameForSession(id) ? "auto" : "none",
    }}>
    <If when={showingFind()}>
      <FindBar query={props.findQuery} count={matchCount} index={matchIndex}
        onQuery={props.findQuery.set} onMove={moveMatch} onClose={() => props.finding.set(false)} />
    </If>
    <SessionTerminal ref={(ref) => { session = ref; }} testID={`terminal-${id}`} sessionId={id} active={store.activeId() === id && !store.inputBlocked() && !props.finding()}
      onFocusRequest={() => { if (!store.inputBlocked() && store.frameForSession(id)) store.select(id); }}
      accessibilityLabel={`Terminal ${tab()?.title ?? id}`}
      fontFamily={monoFont()} fontSize={store.fontSize()}
      lineHeight={store.fontSize() + token("line-gap-terminal")}
      cursorClass="bg-terminal-cursor/55"
      defaultForeground={token("terminal-foreground")} defaultBackground={token("terminal-background")}
      class="min-h-0 flex-1 bg-terminal-background text-terminal-foreground px-(--padding-terminal) pb-(--padding-terminal)"
      onBell={() => { if (store.activeId() !== id) store.patchTab(id, { bell: true }); }}
      onStatusChange={({ info }) => {
        if (info.state === "exited" && info.exitCode === 0 && !info.error) {
          store.closeCleanExit(id);
          return;
        }
        store.patchTab(id, {
          state: info.state, exitCode: info.exitCode, error: info.error,
          ...(!tab()?.customTitle && info.title ? { title: info.title } : {}),
        });
      }} />
    <If when={tab()?.state === "exited" || tab()?.state === "error"}>
      <View class="shrink-0 border-t border-border bg-popover px-3 py-2">
        <Text testID={`session-status-${id}`} class="text-(length:--text-label) text-muted-foreground">
          {tab()?.error ?? `Process exited with code ${tab()?.exitCode ?? "unknown"}. Open a new terminal to continue.`}
        </Text>
      </View>
    </If>
  </View>;
}
