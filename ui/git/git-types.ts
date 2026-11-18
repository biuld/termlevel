import type { GraphGeometry } from "./commit-graph";

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

export type GitDiffMode = "staged" | "unstaged" | "head" | "untracked";

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
  revision?: string;
  unchanged?: boolean;
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
  /** Total decorations when the native query returns a bounded preview. */
  refCount?: number;
  subject: string;
}

export interface GitLog {
  graph?: GraphGeometry[];
  graphMaxLanes?: number;
  graphClipped?: boolean;
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

export interface GitRepository {
  id: number; root: string; gitDir: string; commonDir: string; label: string;
  kind: string;
}
export interface GitDiscoveryPage {
  paths: string[]; repositories?: GitRepository[]; revision?: number; total?: number;
  contextRoots?: Array<{ path: string; root: string }>;
  reset?: boolean; scanning?: boolean; done: boolean; preferredRoot?: string | null; error?: string | null;
}

/** Git data for group views. The native transport runs the system CLI; the preview transport is deterministic. */
export interface GitTransport {
  fork?(): GitTransport;
  cancel?(channel?: string): void;
  dispose?(): void;
  startDiscovery?(root: string, contexts?: string[]): number;
  pollDiscovery?(id: number, revision?: number, offset?: number): GitDiscoveryPage;
  updateDiscovery?(id: number, contexts: string[]): void;
  refreshDiscovery?(id: number): void;
  cancelDiscovery?(id: number): void;
  discover?(root: string): Promise<{ paths: string[]; error?: string | null }>;
  status(root: string): Promise<GitRepoStatus>;
  diff(root: string, path: string, mode: GitDiffMode): Promise<string>;
  branches(root: string): Promise<GitBranches>;
  worktrees(root: string): Promise<GitWorktrees>;
  /** `scope` is a full ref, or empty for every ref. */
  log(root: string, scope: string, limit: number, skip: number): Promise<GitLog>;
  resumeHistory?(root: string, scope: string, count: number): Promise<boolean>;
  matchRefs?(query: string): Promise<string[]>;
  commit(root: string, oid: string): Promise<GitCommitDetail>;
  fetch(root: string): Promise<string>;
  pull(root: string): Promise<string>;
  push(root: string): Promise<string>;
  checkout(root: string, branch: string): Promise<string>;
  createBranch(root: string, name: string, start: string, checkout: boolean): Promise<string>;
  deleteBranch(root: string, name: string): Promise<string>;
  addWorktree(root: string, path: string, branch: string): Promise<string>;
  removeWorktree(root: string, path: string): Promise<string>;
  lockWorktree(root: string, path: string): Promise<string>;
  unlockWorktree(root: string, path: string): Promise<string>;
  pruneWorktrees(root: string): Promise<string>;
}
