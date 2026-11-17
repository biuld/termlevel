import { mount, signal, View } from "nsh";
import { Button } from "nsh/base";
import { HistoryPanel } from "../../../ui/git/HistoryPanel";
import { Type } from "../../../ui/shared/type";
import { createWorkspace } from "../../../ui/workspace/state";
import { parseSnapshot } from "../../../ui/workspace/workspace-snapshot";
import type { GitLog, GitTransport } from "../../../ui/git/git-types";
import { previewGit } from "./preview/git";

mount(() => {
  const root = signal("/alpha");
  const pending = signal(false);
  const disposed = signal(0);
  const visible = signal(true);
  let release: () => void = () => {};
  const snapshot = parseSnapshot(JSON.stringify({ version: 1, groups: [{ id: "group", cwd: "/alpha", title: "Stale reads", color: "purple" }], tabs: [], layouts: {}, workspaceKey: "group" }));
  const workspace = createWorkspace(undefined, { snapshot });
  const create = (): GitTransport => ({ ...previewGit,
    fork: create,
    dispose: () => disposed.update(value => value + 1),
    // Intentionally ignores cancellation: generation checks must still reject the result.
    log: async (path, scope, limit, skip) => {
      const page = await previewGit.log(path, scope, limit, skip);
      const result: GitLog = { ...page, commits: page.commits.map(commit => ({ ...commit, subject: `${path} ${commit.subject}` })) };
      if (path !== "/alpha") return result;
      return new Promise(resolve => { release = () => resolve(result); pending.set(true); });
    },
  });
  return <View class="relative flex-1 flex-col" onLayout={({ bounds }) => workspace.setCanvasSize(bounds.size)}>
    <View class="h-8 shrink-0 flex-row items-center gap-2">
      <Button class="h-8 shrink-0 px-3" testID="switch-repository" onPress={() => root.set("/beta")}>Switch repository</Button>
      <Button class="h-8 shrink-0 px-3" testID="release-old" onPress={() => { release(); pending.set(false); }}>Release old read</Button>
      <Button class="h-8 shrink-0 px-3" testID="close-history" onPress={() => visible.set(false)}>Close history</Button>
      <Type role="caption">{`Pending ${pending()} · Disposed ${disposed()}`}</Type>
    </View>
    <If when={visible()}><HistoryPanel workspace={workspace} root={root} git={create()} preview={false} notify={() => {}} /></If>
  </View>;
});
