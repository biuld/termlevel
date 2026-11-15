import { mount, signal, View } from "nsh";
import { createWorkspace } from "../../../ui/workspace/workspace";
import { Dashboard } from "../../../ui/dashboard/Dashboard";
import { Action } from "../../../ui/shared/controls";
import type { GitRepoStatus, GitTransport } from "../../../ui/dashboard/git-types";

type Mode = "changes" | "clean" | "norepo" | "old";

const base: GitRepoStatus = {
  repo: true,
  root: "/Users/test/project",
  branch: "main",
  detached: false,
  upstream: "origin/main",
  ahead: 1,
  behind: 0,
  operation: null,
  changes: [],
  truncated: false,
  error: null,
};

mount(() => {
  const mode = signal<Mode>("changes");
  const workspace = createWorkspace();
  const git: GitTransport = {
    status: async () => {
      switch (mode()) {
        case "clean": return { ...base };
        case "norepo": return { ...base, repo: false, root: null, branch: null, upstream: null, changes: [] };
        case "old": return { ...base, repo: false, root: null, branch: null, upstream: null, changes: [], error: "Git 2.17.1 is too old. Termlevel needs Git 2.20 or newer." };
        default: return {
          ...base,
          changes: [
            { path: "src/app.rs", status: "M", staged: false, unstaged: true, untracked: false, conflicted: false, kind: "ordinary" },
            { path: "src/both.rs", status: "M", staged: true, unstaged: true, untracked: false, conflicted: false, kind: "ordinary" },
          ],
        };
      }
    },
    diff: async (_root, path, mode) => `--- a/${path}\n+++ b/${path}\n@@ -1 +1 @@\n-old-line\n+${mode}-diff\n`,
    branches: async () => ({ branches: [], error: null }),
    worktrees: async () => ({ worktrees: [], error: null }),
    log: async () => ({ commits: [], hasMore: false, truncated: false, error: null }),
    commit: async () => { throw new Error("No history in this fixture."); },
    checkout: async () => "",
    createBranch: async () => "",
    deleteBranch: async () => "",
    addWorktree: async () => "",
    removeWorktree: async () => "",
    lockWorktree: async () => "",
    unlockWorktree: async () => "",
    pruneWorktrees: async () => "",
  };
  return <View class="flex-1 flex-col">
    <View class="h-10 flex-row items-center gap-1 px-2">
      <Action testID="mode-changes" onPress={() => mode.set("changes")}>Changes</Action>
      <Action testID="mode-clean" onPress={() => mode.set("clean")}>Clean</Action>
      <Action testID="mode-norepo" onPress={() => mode.set("norepo")}>No repo</Action>
      <Action testID="mode-old" onPress={() => mode.set("old")}>Old Git</Action>
    </View>
    <View class="relative min-h-0 flex-1">
      <Dashboard workspace={workspace} preview={false} closeGroup={() => {}} git={git} />
    </View>
  </View>;
});
