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

/** Git data for the overview. The native transport runs the system CLI; the preview transport is deterministic. */
export interface GitTransport {
  status(root: string): Promise<GitRepoStatus>;
  diff(root: string, path: string, mode: GitDiffMode): Promise<string>;
}
