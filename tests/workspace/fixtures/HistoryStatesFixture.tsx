import { mount, View } from "nsh";
import { createWorkspace } from "../../../ui/workspace/workspace";
import { Dashboard } from "../../../ui/dashboard/Dashboard";
import type {
  GitBranch,
  GitCommit,
  GitCommitDetail,
  GitRepoStatus,
  GitTransport,
  GitWorktree,
} from "../../../ui/dashboard/git-types";

const root = "/Users/test/project";

const commits: GitCommit[] = [
  {
    oid: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    short: "aaaaaaa",
    parents: ["bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "cccccccccccccccccccccccccccccccccccccccc"],
    author: "Mira Chen",
    email: "mira@example.com",
    timestamp: 1_763_500_800,
    refs: ["HEAD -> main"],
    subject: "Merge feature/history",
  },
  {
    oid: "cccccccccccccccccccccccccccccccccccccccc",
    short: "ccccccc",
    parents: ["bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"],
    author: "Mira Chen",
    email: "mira@example.com",
    timestamp: 1_763_500_200,
    refs: ["feature/history"],
    subject: "Group commits by lane",
  },
  {
    oid: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    short: "bbbbbbb",
    parents: ["dddddddddddddddddddddddddddddddddddddddd"],
    author: "Tyler Chan",
    email: "tyler@example.com",
    timestamp: 1_763_499_600,
    refs: [],
    subject: "Show commit details and patch",
  },
  {
    oid: "dddddddddddddddddddddddddddddddddddddddd",
    short: "ddddddd",
    parents: [],
    author: "Tyler Chan",
    email: "tyler@example.com",
    timestamp: 1_763_498_000,
    refs: ["tag: v0.1"],
    subject: "Initial commit",
  },
];

const branches: GitBranch[] = [
  { name: "main", fullName: "refs/heads/main", kind: "local", current: true, upstream: "origin/main", target: commits[0].oid },
  { name: "feature/history", fullName: "refs/heads/feature/history", kind: "local", current: false, upstream: null, target: commits[1].oid },
  { name: "origin/main", fullName: "refs/remotes/origin/main", kind: "remote", current: false, upstream: null, target: commits[2].oid },
  { name: "v0.1", fullName: "refs/tags/v0.1", kind: "tag", current: false, upstream: null, target: commits[3].oid },
];

const worktrees: GitWorktree[] = [
  { path: root, branch: "main", head: commits[0].oid, bare: false, detached: false, locked: null, prunable: null, main: true, current: true },
  { path: `${root}-wt/history`, branch: "feature/history", head: commits[1].oid, bare: false, detached: false, locked: null, prunable: null, main: false, current: false },
];

const status: GitRepoStatus = {
  repo: true,
  root,
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

function detail(oid: string): GitCommitDetail {
  const commit = commits.find((item) => item.oid === oid) ?? commits[0];
  return {
    commit,
    body: `History panel fixture for ${commit.short}.`,
    files: [
      { path: "ui/dashboard/HistoryPanel.tsx", status: "A" },
      { path: "src/git.rs", status: "M" },
    ],
    patch: [
      "--- a/ui/dashboard/HistoryPanel.tsx",
      "+++ b/ui/dashboard/HistoryPanel.tsx",
      "@@ -1,1 +1,2 @@",
      " import { View } from \"nsh\";",
      "+import { DataTable } from \"nsh/base\";",
    ].join("\n"),
    truncated: false,
    error: null,
  };
}

mount(() => {
  const workspace = createWorkspace();
  let branchList: GitBranch[] = [...branches];
  let worktreeList: GitWorktree[] = [...worktrees];
  const git: GitTransport = {
    status: async () => ({ ...status }),
    diff: async () => "",
    branches: async () => ({ branches: branchList, error: null }),
    worktrees: async () => ({ worktrees: worktreeList, error: null }),
    log: async () => ({ commits, hasMore: false, truncated: false, error: null }),
    commit: async (_root, oid) => detail(oid),
    checkout: async () => "",
    createBranch: async (_root, name, _start, checkout) => {
      if (checkout) branchList = branchList.map((branch) => branch.kind === "local" ? { ...branch, current: false } : branch);
      branchList = [...branchList, {
        name,
        fullName: `refs/heads/${name}`,
        kind: "local",
        current: checkout,
        upstream: null,
        target: commits[0].oid,
      }];
      return "";
    },
    deleteBranch: async (_root, name) => {
      branchList = branchList.filter((branch) => !(branch.kind === "local" && branch.name === name));
      return "";
    },
    addWorktree: async (_root, path, branch) => {
      worktreeList = [...worktreeList, {
        path,
        branch,
        head: commits[0].oid,
        bare: false,
        detached: false,
        locked: null,
        prunable: null,
        main: false,
        current: false,
      }];
      return path;
    },
    removeWorktree: async (_root, path) => {
      worktreeList = worktreeList.filter((worktree) => worktree.path !== path);
      return "";
    },
    lockWorktree: async () => "",
    unlockWorktree: async () => "",
    pruneWorktrees: async () => "",
  };
  return <View class="relative flex-1" onLayout={({ bounds }) => workspace.setCanvasSize(bounds.size)}>
    <Dashboard workspace={workspace} preview={false} closeGroup={() => {}} git={git} />
  </View>;
});
