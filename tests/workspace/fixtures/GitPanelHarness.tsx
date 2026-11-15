import { signal, View } from "nsh";
import { Button } from "nsh/base";
import { GitPanel } from "../../../ui/git/GitPanel";
import { HistoryPanel } from "../../../ui/git/HistoryPanel";
import type { GitTransport } from "../../../ui/git/git-types";
import type { Workspace } from "../../../ui/workspace/state";
import { Type } from "../../../ui/shared/type";

/** Exercises the panels independently of application navigation. */
export function GitPanelHarness(props: { workspace: Workspace; git: GitTransport }) {
  const view = signal<"changes" | "history">("changes");
  const notice = signal("");
  return <View testID="dashboard" class="absolute inset-0 flex-col">
    <View class="h-10 shrink-0 flex-row">
      <Button class="h-10 px-3" testID="dashboard-view-changes" onPress={() => view.set("changes")}>Changes</Button>
      <Button class="h-10 px-3" testID="dashboard-view-history" onPress={() => view.set("history")}>History</Button>
    </View>
    <If when={view() === "changes"}><GitPanel workspace={props.workspace} git={props.git} preview={false} /></If>
    <If when={!!notice()}><Type testID="fixture-notice" role="caption">{notice()}</Type></If>
    <If when={view() === "history"}><HistoryPanel workspace={props.workspace} git={props.git} preview={false} notify={(_level, title, body) => notice.set(`${title} · ${body}`)} /></If>
  </View>;
}
