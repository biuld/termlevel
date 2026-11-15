import { Git, GitOps } from "termlevel/native";
import type { GitTransport } from "./git-types";

/** Real Git data from the system CLI through the Rust read-only module. */
export const nativeGit: GitTransport = {
  status: (root) => Git.status(root),
  diff: (root, path, mode) => Git.diff(root, path, mode),
  branches: async (root) => {
    const result = await Git.branches(root);
    return {
      branches: result.branches.map((branch) => ({
        ...branch,
        kind: branch.kind as "local" | "remote" | "tag",
      })),
      error: result.error,
    };
  },
  worktrees: (root) => Git.worktrees(root),
  log: (root, scope, limit, skip) => Git.log(root, scope, limit, skip),
  commit: (root, oid) => Git.commit(root, oid),
  fetch: (root) => GitOps.fetch(root),
  pull: (root) => GitOps.pull(root),
  checkout: (root, branch) => GitOps.checkout(root, branch),
  createBranch: (root, name, start, checkout) => GitOps.createBranch(root, name, start, checkout),
  deleteBranch: (root, name) => GitOps.deleteBranch(root, name),
  addWorktree: (root, path, branch) => GitOps.addWorktree(root, path, branch),
  removeWorktree: (root, path) => GitOps.removeWorktree(root, path),
  lockWorktree: (root, path) => GitOps.lockWorktree(root, path),
  unlockWorktree: (root, path) => GitOps.unlockWorktree(root, path),
  pruneWorktrees: (root) => GitOps.pruneWorktrees(root),
};
