import { effect, signal, token, View, type Signal } from "nsh";
import { SessionTerminal, type SessionTerminalRef } from "termlevel/native";
import { monoFont } from "../shared/fonts";
import type { Workspace } from "../workspace/workspace";
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
  const searching = signal(false);
  const showingFind = () => props.finding() && store.activeId() === id;
  effect(() => {
    const query = props.findQuery();
    const open = showingFind();
    const current = session;
    if (!open) {
      current?.clearSearch();
      matchCount.set(0);
      matchIndex.set(0);
      searching.set(false);
      return;
    }
    void (async () => {
      const found = await current?.search(query) ?? 0;
      if (props.findQuery() !== query || !(props.finding() && store.activeId() === id)) return;
      // SearchChange also reports completion after this initial bounded step.
      if (found) { matchCount.set(found); matchIndex.set(1); }
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
      <FindBar query={props.findQuery} count={matchCount} index={matchIndex} searching={searching}
        onQuery={props.findQuery.set} onMove={moveMatch} onClose={() => props.finding.set(false)} />
    </If>
    <SessionTerminal ref={(ref) => { session = ref; }} testID={`terminal-${id}`} sessionId={id} active={store.activeId() === id && !store.inputBlocked() && !props.finding()}
      visible={!!store.frameForSession(id)}
      inputBlocked={store.inputBlocked() || props.finding()}
      onFocusRequest={() => { if (!store.inputBlocked() && store.frameForSession(id)) store.select(id); }}
      accessibilityLabel={`Terminal ${tab()?.title ?? id}`}
      fontFamily={monoFont()} fontSize={store.fontSize()}
      lineHeight={store.fontSize() + token("line-gap-terminal")}
      cursorClass="bg-terminal-cursor/55"
      defaultForeground={token("terminal-foreground")} defaultBackground={token("terminal-background")}
      class="min-h-0 flex-1 bg-terminal-background text-terminal-foreground px-(--padding-terminal) pb-(--padding-terminal)"
      onSearchChange={({ result }) => {
        if (!showingFind() || result.query !== props.findQuery()) return;
        matchCount.set(result.count);
        matchIndex.set(result.index);
        searching.set(result.searching);
      }}
      onBell={() => { if (store.activeId() !== id) store.patchTab(id, { bell: true }); }}
      onStatusChange={({ info }) => {
        if (info.state === "exited" && info.exitCode === 0 && !info.error) {
          store.closeCleanExit(id);
          return;
        }
        store.patchTab(id, {
          state: info.state, exitCode: info.exitCode, error: info.error, inputWarning: info.inputWarning,
          ...(!tab()?.customTitle && info.title ? { title: info.title } : {}),
        });
      }} />
  </View>;
}
