import { cleanup, mount, signal, View } from "nsh";
import { Button } from "nsh/base";
import { HistoryPanel } from "../../../ui/git/HistoryPanel";
import type { GitBranch, GitCommit, GitTransport } from "../../../ui/git/git-types";
import { createWorkspace } from "../../../ui/workspace/state";
import { parseSnapshot } from "../../../ui/workspace/workspace-snapshot";
import { Type } from "../../../ui/shared/type";
import { previewGit } from "./preview/git";

const longName = "feature/history-rendering-and-large-repository-performance/keep-the-distinguishing-suffix";
const commits: GitCommit[] = Array.from({ length: 4000 }, (_, index) => ({
  oid: index.toString(16).padStart(40, "0"), short: index.toString(16).padStart(7, "0"),
  parents: index === 3999 ? [] : [(index + 1).toString(16).padStart(40, "0")],
  author: "Test", email: "test@example.com", timestamp: 1_700_000_000 - index,
  refs: index === 0 ? [`HEAD -> ${longName}`, `origin/${longName}`, "tag: release/2026.11.17-with-a-very-long-release-name"] : [],
  subject: `Scale commit ${index}`,
}));
const branches: GitBranch[] = Array.from({ length: 2000 }, (_, index) => ({
  name: index === 0 ? longName : `origin/${longName}-${index}`,
  fullName: index === 0 ? `refs/heads/${longName}` : `refs/remotes/origin/${longName}-${index}`,
  kind: index === 0 ? "local" : "remote", current: index === 0, target: commits[0].oid,
}));

mount(() => {
  const snapshot = parseSnapshot(JSON.stringify({ version: 1, groups: [{ id: "group", title: "Scale", cwd: "/scale", color: "purple" }], tabs: [], layouts: {}, workspaceKey: "group" }));
  const workspace = createWorkspace(undefined, { snapshot });
  const tick = signal(0);
  const deliveries = signal(0);
  const unchanged = signal(0);
  const logs = signal(0);
  const heartbeats = signal(0);
  const timer = setInterval(() => heartbeats.update(value => value + 1), 20);
  cleanup(() => clearInterval(timer));
  const git: GitTransport = { ...previewGit,
    status: async root => ({ ...await previewGit.status(root), branch: longName }),
    branches: async () => {
      if (deliveries()) { unchanged.update(value => value + 1); return { revision: "same", unchanged: true, branches: [] }; }
      deliveries.update(value => value + 1);
      return { revision: "same", branches };
    },
    log: async () => { logs.update(value => value + 1); return { commits, hasMore: false, truncated: false }; },
    commit: async (root, oid) => ({ ...await previewGit.commit(root, oid), commit: commits.find(commit => commit.oid === oid)! }),
  };
  return <View class="relative flex-1 flex-col" onLayout={({ bounds }) => workspace.setCanvasSize(bounds.size)}>
    <View class="h-8 shrink-0 flex-row items-center gap-3 px-3">
      <Button class="h-8 shrink-0 px-3" testID="scale-refresh" onPress={() => tick.update(value => value + 1)}>Refresh refs</Button>
      <Type role="caption">{`Snapshots ${deliveries()} · Unchanged ${unchanged()} · Logs ${logs()} · Heartbeats ${heartbeats()}`}</Type>
    </View>
    <HistoryPanel workspace={workspace} git={git} preview={false} refreshTick={tick} notify={() => {}} />
  </View>;
});
