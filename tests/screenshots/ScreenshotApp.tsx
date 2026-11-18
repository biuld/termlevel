import { mount } from "nsh";
import { WorkspaceApp } from "../../ui/app/WorkspaceApp";
import { createWorkspace } from "../../ui/workspace/state";
import { PreviewTerminal } from "../workspace/fixtures/preview/Terminal";
import type {
  GitBranch,
  GitCommit,
  GitCommitDetail,
  GitRepoStatus,
  GitTransport,
  GitWorktree,
} from "../../ui/git/git-types";
import type { DirectoryListing, FileText, FilesTransport } from "../../ui/files/types";

const gitRoot = "/work/termlevel-demo";

const commits: GitCommit[] = [
  {
    oid: "9f3c1a20b7d4e6f8091a2b3c4d5e6f708192a3b4",
    short: "9f3c1a2",
    parents: ["4d2b8c11a5e7f90123456789abcdef0123456789a", "7a1e9c3355ddaabbccddeeff0011223344556677"],
    author: "Avery Lin",
    email: "avery@example.com",
    timestamp: 1_763_500_800,
    refs: ["HEAD -> main"],
    subject: "Merge feature/tiling",
  },
  {
    oid: "7a1e9c3355ddaabbccddeeff0011223344556677",
    short: "7a1e9c3",
    parents: ["4d2b8c11a5e7f90123456789abcdef0123456789a"],
    author: "Sam Rivera",
    email: "sam@example.com",
    timestamp: 1_763_500_200,
    refs: ["feature/tiling"],
    subject: "Keep pane focus and geometry stable",
  },
  {
    oid: "4d2b8c11a5e7f90123456789abcdef0123456789a",
    short: "4d2b8c1",
    parents: ["2b6f0d99c3a1e5f708192a3b4c5d6e7f8091a2b3"],
    author: "Avery Lin",
    email: "avery@example.com",
    timestamp: 1_763_499_600,
    refs: [],
    subject: "Group directories and per-group tools",
  },
  {
    oid: "2b6f0d99c3a1e5f708192a3b4c5d6e7f8091a2b3",
    short: "2b6f0d9",
    parents: ["c0ffee00a1b2c3d4e5f60718293a4b5c6d7e8f90"],
    author: "Sam Rivera",
    email: "sam@example.com",
    timestamp: 1_763_498_900,
    refs: ["origin/main"],
    subject: "Read branches, tags and worktrees",
  },
  {
    oid: "c0ffee00a1b2c3d4e5f60718293a4b5c6d7e8f90",
    short: "c0ffee0",
    parents: [],
    author: "Avery Lin",
    email: "avery@example.com",
    timestamp: 1_763_498_000,
    refs: ["tag: v0.1"],
    subject: "Initial workspace preview",
  },
];

const branches: GitBranch[] = [
  { name: "main", fullName: "refs/heads/main", kind: "local", current: true, upstream: "origin/main", target: commits[0].oid },
  { name: "feature/tiling", fullName: "refs/heads/feature/tiling", kind: "local", current: false, upstream: null, target: commits[1].oid },
  { name: "origin/main", fullName: "refs/remotes/origin/main", kind: "remote", current: false, upstream: null, target: commits[3].oid },
  { name: "v0.1", fullName: "refs/tags/v0.1", kind: "tag", current: false, upstream: null, target: commits[4].oid },
];

const worktrees: GitWorktree[] = [
  { path: gitRoot, branch: "main", head: commits[0].oid, bare: false, detached: false, main: true, current: true },
  { path: `${gitRoot}-wt/tiling`, branch: "feature/tiling", head: commits[1].oid, bare: false, detached: false, main: false, current: false },
];

const status: GitRepoStatus = {
  repo: true,
  root: gitRoot,
  branch: "main",
  detached: false,
  upstream: "origin/main",
  ahead: 2,
  behind: 0,
  operation: null,
  changes: [
    { path: "docs/README.md", status: "M", staged: false, unstaged: true, untracked: false, conflicted: false, kind: "ordinary" },
    { path: "ui/workspace/Sidebar.tsx", status: "M", staged: false, unstaged: true, untracked: false, conflicted: false, kind: "ordinary" },
    { path: "src/session_view/surface.rs", status: "M", staged: true, unstaged: false, untracked: false, conflicted: false, kind: "ordinary" },
    { path: "docs/design/tiling.md", status: "M", staged: true, unstaged: true, untracked: false, conflicted: false, kind: "ordinary" },
    { path: "tests/workspace/Tiling.test.tsx", status: "?", staged: false, unstaged: false, untracked: true, conflicted: false, kind: "untracked" },
  ],
  truncated: false,
  error: null,
};

const changePatch = `--- a/docs/README.md
+++ b/docs/README.md
@@ -1,8 +1,11 @@
 # Documentation

-This directory documents the current state of the project.
+This directory documents how Termlevel is built, designed and verified.

+The product README is the user-facing introduction.
+Architecture, design and validation have separate owners.
+
 ## Map

-| Document | Purpose |
-|---|---|
+| Document | Status | Purpose |
+|---|---|---|
`;

function detail(oid: string): GitCommitDetail {
  const commit = commits.find((item) => item.oid === oid) ?? commits[0];
  return {
    commit,
    body: "Keep the workspace stable while panes move. The commit records the preview data used by the screenshot fixture; no Git command ran.",
    files: [
      { path: "ui/workspace/TilingCanvas.tsx", status: "M" },
      { path: "ui/workspace/tiling.ts", status: "M" },
      { path: "tests/workspace/Tiling.test.tsx", status: "A" },
    ],
    patch: `--- a/ui/workspace/TilingCanvas.tsx
+++ b/ui/workspace/TilingCanvas.tsx
@@ -1,3 +1,5 @@
 import { View } from "nsh";
+import { TerminalToolbar } from "./TerminalToolbar";
+import { TerminalSurface } from "./TerminalSurface";

 export function Pane() {
`,
    truncated: false,
    error: null,
  };
}

const demoGit: GitTransport = {
  status: async () => status,
  diff: async (_root, path, mode) => (path === "docs/README.md" ? changePatch : `--- a/${path}\n+++ b/${path}\n@@ -1 +1 @@\n-old value\n+${mode} value\n`),
  branches: async () => ({ branches, error: null }),
  worktrees: async () => ({ worktrees, error: null }),
  log: async () => ({ commits, hasMore: false, truncated: false, error: null }),
  commit: async (_root, oid) => detail(oid),
  fetch: async () => "Fetched.",
  pull: async () => "Pulled.",
  push: async () => "Pushed.",
  checkout: async () => "Checked out.",
  createBranch: async () => "Branch created.",
  deleteBranch: async () => "Branch deleted.",
  addWorktree: async () => "Worktree created.",
  removeWorktree: async () => "Worktree removed.",
  lockWorktree: async () => "Worktree locked.",
  unlockWorktree: async () => "Worktree unlocked.",
  pruneWorktrees: async () => "Worktrees pruned.",
};

const readme = `# Documentation

This directory documents how Termlevel is built, designed and verified.
The product README is the user-facing introduction.

## Map

| Document | Purpose |
|---|---|
| development.md | Setup, ownership and validation |
| architecture/ | Current data paths and lifecycle |
| design/ | UI, rendering and interaction rules |
| roadmap.md | Outstanding work |
| validation/ | Dated evidence |

## Status

Current documents describe the code as it exists today.
Evidence documents are append-only.

## Maintenance

Update the owning document in the same change as the code.
Link to the owner instead of copying facts.
`;

const fileText = (text: string): FileText => ({
  text,
  binary: false,
  truncated: false,
  lossy: false,
  bytes: text.length,
  modifiedMs: 1_700_000_000_000,
});

function listing(names: Array<[string, "directory" | "file", number?]>): DirectoryListing {
  return {
    entries: names.map(([name, kind, bytes]) => ({ name, kind, hidden: name.startsWith("."), bytes: kind === "file" ? (bytes ?? 512) : null })),
    truncated: false,
  };
}

const trees: Record<string, DirectoryListing> = {
  "": listing([["docs", "directory"], ["src", "directory"], [".gitignore", "file", 120], ["README.md", "file", readme.length], ["theme.json", "file", 8090]]),
  docs: listing([["architecture", "directory"], ["design", "directory"], ["validation", "directory"], ["README.md", "file"], ["development.md", "file"], ["roadmap.md", "file"]]),
  "docs/architecture": listing([["README.md", "file"], ["terminal.md", "file"], ["workspace.md", "file"], ["git.md", "file"], ["files.md", "file"], ["notifications.md", "file"]]),
  "docs/design": listing([["terminal.md", "file"], ["ui.md", "file"], ["osc-status.md", "file"], ["tiling.md", "file"]]),
  src: listing([["files", "directory"], ["sessions", "directory"], ["session_view", "directory"], ["lib.rs", "file"]]),
  "src/session_view": listing([["platform", "directory"], ["surface.rs", "file"], ["presentation.rs", "file"], ["view.rs", "file"]]),
  "src/files": listing([["mod.rs", "file"], ["list.rs", "file"], ["content.rs", "file"], ["watch.rs", "file"]]),
  "src/sessions": listing([["api.rs", "file"], ["input_queue.rs", "file"]]),
};

const demoFiles: FilesTransport = {
  list: async (_root, directory) => trees[directory] ?? listing([]),
  read: async (_root, path) => {
    if (path === "README.md") return fileText(readme);
    return fileText(`# ${path.split("/").pop()}\n\nDemo content for the screenshot fixture.\n\n- No user files are read.\n- No commands are executed.\n- The tree is deterministic.\n`);
  },
  save: async (_root, path, text) => fileText(text),
  absolute: async (_root, path) => `${gitRoot}/${path}`,
  terminalDirectory: async (_root, path) => path.includes("/") ? `${gitRoot}/${path.slice(0, path.lastIndexOf("/"))}` : gitRoot,
  reveal: async () => {},
  openWithDefaultApp: async () => {},
  trash: async () => {},
};

mount(() => {
  const workspace = createWorkspace();
  return <WorkspaceApp workspace={workspace} git={demoGit} files={demoFiles} previewSurface={PreviewTerminal} />;
});
