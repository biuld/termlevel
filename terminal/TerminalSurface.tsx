import { Text, token, View } from "nsh";
import { SessionTerminal } from "termlevel/native";
import { monoFont } from "../ui/fonts";
import type { Workspace } from "../ui/workspace";
import { PreviewTerminal } from "./PreviewTerminal";

export function TerminalSurface(props: { id: string; workspace: Workspace; preview: boolean }) {
  if (props.preview) return <PreviewTerminal id={props.id} workspace={props.workspace} />;
  const store = props.workspace;
  const id = props.id;
  const tab = () => store.tab(id);
  return <View testID={`surface-${id}`}
    accessibilityElementsHidden={!store.frameForSession(id)}
    style={{
      position: "absolute", top: 0, right: 0, bottom: 0, left: 0,
      opacity: store.frameForSession(id) ? 1 : 0,
      pointerEvents: store.frameForSession(id) ? "auto" : "none",
    }}>
    <SessionTerminal testID={`terminal-${id}`} sessionId={id} active={store.activeId() === id && !store.inputBlocked()}
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
