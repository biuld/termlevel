/** One changed path from Git, independent of whether it came from the CLI or the demo. */
export interface GitChange {
  path: string;
  oldPath?: string | null;
  /** One status letter: M, A, D, R, C, T, U or ?. */
  status: string;
  staged: boolean;
  unstaged: boolean;
  untracked: boolean;
  conflicted: boolean;
  kind: string;
}

export interface GitRepoStatus {
  repo: boolean;
  root?: string | null;
  branch?: string | null;
  detached: boolean;
  upstream?: string | null;
  ahead: number;
  behind: number;
  operation?: string | null;
  changes: GitChange[];
  truncated: boolean;
  error?: string | null;
}

export type GitDiffMode = "staged" | "unstaged" | "untracked";

/** One local branch, remote-tracking branch or tag. */
export interface GitBranch {
  /** The short name: `main`, `origin/main` or a tag name. */
  name: string;
  /** The full ref, and the history scope the log accepts. */
  fullName: string;
  kind: "local" | "remote" | "tag";
  current: boolean;
  upstream?: string | null;
  target: string;
}

export interface GitBranches {
  branches: GitBranch[];
  error?: string | null;
}

/** One worktree of the repository. */
export interface GitWorktree {
  path: string;
  branch?: string | null;
  head?: string | null;
  bare: boolean;
  detached: boolean;
  locked?: string | null;
  prunable?: string | null;
  /** The repository's first worktree. */
  main: boolean;
  /** The worktree that holds the group folder. */
  current: boolean;
}

export interface GitWorktrees {
  worktrees: GitWorktree[];
  error?: string | null;
}

/** One commit in the history list. */
export interface GitCommit {
  oid: string;
  short: string;
  parents: string[];
  author: string;
  email: string;
  /** Author time, seconds since the Unix epoch. */
  timestamp: number;
  /** Decorations such as `HEAD -> main`, `origin/main` or `tag: v1`. */
  refs: string[];
  subject: string;
}

export interface GitLog {
  commits: GitCommit[];
  hasMore: boolean;
  truncated: boolean;
  error?: string | null;
}

export interface GitCommitFile {
  path: string;
  oldPath?: string | null;
  status: string;
}

export interface GitCommitDetail {
  commit: GitCommit;
  body: string;
  files: GitCommitFile[];
  /** A unified patch against the first parent. */
  patch: string;
  truncated: boolean;
  error?: string | null;
}

/** Git data for the overview. The native transport runs the system CLI; the preview transport is deterministic. */
export interface GitTransport {
  status(root: string): Promise<GitRepoStatus>;
  diff(root: string, path: string, mode: GitDiffMode): Promise<string>;
  branches(root: string): Promise<GitBranches>;
  worktrees(root: string): Promise<GitWorktrees>;
  /** `scope` is a full ref, or empty for every ref. */
  log(root: string, scope: string, limit: number, skip: number): Promise<GitLog>;
  commit(root: string, oid: string): Promise<GitCommitDetail>;
  checkout(root: string, branch: string): Promise<string>;
  createBranch(root: string, name: string, start: string, checkout: boolean): Promise<string>;
  deleteBranch(root: string, name: string): Promise<string>;
  addWorktree(root: string, path: string, branch: string): Promise<string>;
  removeWorktree(root: string, path: string): Promise<string>;
  lockWorktree(root: string, path: string): Promise<string>;
  unlockWorktree(root: string, path: string): Promise<string>;
  pruneWorktrees(root: string): Promise<string>;
}
