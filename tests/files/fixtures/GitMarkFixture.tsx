import { mount, signal, View } from "nsh";
import { Button } from "nsh/base";
import { FilesWorkspace } from "../../../ui/files/FilesWorkspace";
import type { FilesTransport } from "../../../ui/files/types";
import type { GitRepoStatus, GitTransport } from "../../../ui/git/git-types";
import { createWorkspace } from "../../../ui/workspace/state";

const status: GitRepoStatus = {
  repo: true,
  root: "/proj",
  branch: "main",
  detached: false,
  upstream: null,
  ahead: 0,
  behind: 0,
  operation: null,
  truncated: false,
  error: null,
  changes: [
    { path: "src/main.ts", status: "M", staged: false, unstaged: true, untracked: false, conflicted: false, kind: "ordinary" },
    { path: "readme.md", status: "M", staged: true, unstaged: false, untracked: false, conflicted: false, kind: "ordinary" },
  ],
};

mount(() => {
  const workspace = createWorkspace();
  workspace.openFiles("termlevel");
  const release = signal(false);
  let openGate: () => void = () => {};
  const gate = new Promise<void>((resolve) => { openGate = resolve; });
  const files: FilesTransport = {
    list: async (_root, directory) => directory === ""
      ? { entries: [
        { name: "src", kind: "directory", hidden: false },
        { name: "readme.md", kind: "file", hidden: false },
      ], truncated: false }
      : { entries: [{ name: "main.ts", kind: "file", hidden: false }], truncated: false },
    read: async () => ({ text: "fn", binary: false, truncated: false, lossy: false, bytes: 2, modifiedMs: 1 }),
    save: async (_root, _path, value) => ({ text: value, binary: false, truncated: false, lossy: false, bytes: value.length, modifiedMs: 2 }),
    absolute: async () => "/proj",
    terminalDirectory: async () => "/proj",
    reveal: async () => {},
    openWithDefaultApp: async () => {},
    trash: async () => {},
  };
  const git = {
    status: async () => {
      await gate;
      return status;
    },
    diff: async () => "",
    branches: async () => ({ branches: [], error: null }),
    worktrees: async () => ({ worktrees: [], error: null }),
    log: async () => ({ commits: [], hasMore: false, truncated: false, error: null }),
    commit: async () => { throw new Error("unused"); },
    fetch: async () => "",
    pull: async () => "",
    push: async () => "",
    checkout: async () => "",
    createBranch: async () => "",
    deleteBranch: async () => "",
    addWorktree: async () => "",
    removeWorktree: async () => "",
    lockWorktree: async () => "",
    unlockWorktree: async () => "",
    pruneWorktrees: async () => "",
  } satisfies GitTransport;
  return <View testID="git-mark-fixture" class="absolute inset-0 flex-col">
    <Button testID="release-git" class="h-8" onPress={() => { if (!release()) { release.set(true); openGate(); } }}>Release Git</Button>
    <View class="min-h-0 flex-1">
      <FilesWorkspace workspace={workspace} files={files} git={git} notify={() => {}} />
    </View>
  </View>;
});
