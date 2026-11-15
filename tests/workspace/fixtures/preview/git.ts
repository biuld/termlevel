import type {
  GitBranch,
  GitCommit,
  GitCommitDetail,
  GitLog,
  GitRepoStatus,
  GitTransport,
  GitWorktree,
} from "../../../../ui/git/git-types";

const commits: GitCommit[] = [
  {
    oid: "9f3c1a20b7d4e6f8091a2b3c4d5e6f708192a3b4",
    short: "9f3c1a2",
    parents: [
      "4d2b8c11a5e7f90123456789abcdef0123456789a",
      "7a1e9c3355ddaabbccddeeff0011223344556677",
    ],
    author: "Mira Chen",
    email: "mira@example.com",
    timestamp: 1_763_500_800,
    refs: ["HEAD -> main"],
    subject: "Merge feature/history",
  },
  {
    oid: "7a1e9c3355ddaabbccddeeff0011223344556677",
    short: "7a1e9c3",
    parents: ["4d2b8c11a5e7f90123456789abcdef0123456789a"],
    author: "Mira Chen",
    email: "mira@example.com",
    timestamp: 1_763_500_200,
    refs: ["feature/history"],
    subject: "Group commits by lane",
  },
  {
    oid: "4d2b8c11a5e7f90123456789abcdef0123456789a",
    short: "4d2b8c1",
    parents: ["2b6f0d99c3a1e5f708192a3b4c5d6e7f8091a2b3"],
    author: "Tyler Chan",
    email: "tyler@example.com",
    timestamp: 1_763_499_600,
    refs: [],
    subject: "Show commit details and patch",
  },
  {
    oid: "2b6f0d99c3a1e5f708192a3b4c5d6e7f8091a2b3",
    short: "2b6f0d9",
    parents: ["c0ffee00a1b2c3d4e5f60718293a4b5c6d7e8f90"],
    author: "Tyler Chan",
    email: "tyler@example.com",
    timestamp: 1_763_498_900,
    refs: ["origin/main"],
    subject: "Read branches and worktrees",
  },
  {
    oid: "c0ffee00a1b2c3d4e5f60718293a4b5c6d7e8f90",
    short: "c0ffee0",
    parents: [],
    author: "Tyler Chan",
    email: "tyler@example.com",
    timestamp: 1_763_498_000,
    refs: ["tag: v0.1"],
    subject: "Initial commit",
  },
];

const branches: GitBranch[] = [
  {
    name: "main",
    fullName: "refs/heads/main",
    kind: "local",
    current: true,
    upstream: "origin/main",
    target: commits[0].oid,
  },
  {
    name: "feature/history",
    fullName: "refs/heads/feature/history",
    kind: "local",
    current: false,
    upstream: null,
    target: commits[1].oid,
  },
  {
    name: "origin/main",
    fullName: "refs/remotes/origin/main",
    kind: "remote",
    current: false,
    upstream: null,
    target: commits[3].oid,
  },
  {
    name: "v0.1",
    fullName: "refs/tags/v0.1",
    kind: "tag",
    current: false,
    upstream: null,
    target: commits[4].oid,
  },
];

function worktrees(root: string): GitWorktree[] {
  const base = root.replace(/[\\/]+$/, "");
  return [
    {
      path: base,
      branch: "main",
      head: commits[0].oid,
      bare: false,
      detached: false,
      locked: null,
      prunable: null,
      main: true,
      current: true,
    },
    {
      path: `${base}-wt/history`,
      branch: "feature/history",
      head: commits[1].oid,
      bare: false,
      detached: false,
      locked: null,
      prunable: null,
      main: false,
      current: false,
    },
  ];
}

function detail(oid: string): GitCommitDetail {
  const commit = commits.find((item) => item.oid === oid) ?? commits[0];
  return {
    commit,
    body: "Preview data for the history panel. No Git command ran.",
    files: [
      { path: "ui/git/HistoryPanel/panel.tsx", status: "A" },
      { path: "ui/git/commit-graph.ts", status: "A" },
      { path: "src/git.rs", status: "M" },
    ],
    patch: [
      "--- a/ui/git/HistoryPanel/panel.tsx",
      "+++ b/ui/git/HistoryPanel/panel.tsx",
      "@@ -1,3 +1,7 @@",
      " import { View } from \"nsh\";",
      "+import { DataTable } from \"nsh/base\";",
      "+import { layoutCommits } from \"./commit-graph\";",
      " ",
      "-export function HistoryPanel() { return <View />; }",
      "+export function HistoryPanel() {",
      "+  return <DataTable columns={[]} data={[]} />;",
      "+}",
    ].join("\n"),
    truncated: false,
    error: null,
  };
}

const fail = async (): Promise<string> => {
  throw new Error("Preview mode does not change Git.");
};

/** Deterministic preview: a clean repository, never starts git or reads storage. */
export const previewGit: GitTransport = {
  status: async (root): Promise<GitRepoStatus> => ({
    repo: true,
    root,
    branch: "main",
    detached: false,
    upstream: "origin/main",
    ahead: 0,
    behind: 0,
    operation: null,
    changes: [],
    truncated: false,
    error: null,
  }),
  diff: async () => "",
  branches: async () => ({ branches, error: null }),
  worktrees: async (root) => ({ worktrees: worktrees(root), error: null }),
  log: async (): Promise<GitLog> => ({
    commits,
    hasMore: false,
    truncated: false,
    error: null,
  }),
  commit: async (_root, oid) => detail(oid),
  fetch: fail,
  pull: fail,
  checkout: fail,
  createBranch: fail,
  deleteBranch: fail,
  addWorktree: fail,
  removeWorktree: fail,
  lockWorktree: fail,
  unlockWorktree: fail,
  pruneWorktrees: fail,
};
