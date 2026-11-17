import { Git, GitOps, GitDiscovery, GitQueries } from "termlevel/native";
import type { GitBranch, GitTransport } from "./git-types";

/** Each fork is a read lease. Importing the module (including preview) starts no work. */
function createNativeGit(): GitTransport {
  let owner: number | null = null;
  let disposed = false;
  let generation = 0;
  let repository = "";
  let revision = "";
  function selectRoot(root: string) {
    if (root !== repository) { repository = root; revision = ""; }
  }
  const lease = () => {
    if (disposed) throw new Error("Git view closed.");
    return owner ??= GitQueries.open();
  };
  return {
    fork: createNativeGit,
    cancel: (channel) => {
      if (channel) { if (owner !== null) GitQueries.cancelRead(owner, channel); return; }
      generation++;
      if (owner !== null) GitQueries.cancel(owner);
    },
    dispose: () => {
      disposed = true;
      generation++;
      if (owner !== null) GitQueries.close(owner);
      owner = null;
      revision = "";
    },
    startDiscovery: (root, contexts = []) => GitDiscovery.start(root, contexts),
    pollDiscovery: (id, revision = 0, offset = 0) => GitDiscovery.poll(id, revision, offset),
    updateDiscovery: (id, contexts) => GitDiscovery.update(id, contexts),
    refreshDiscovery: (id) => GitDiscovery.refresh(id),
    cancelDiscovery: (id) => GitDiscovery.cancel(id),
    status: (root) => { selectRoot(root); return Git.status(root, lease()); },
    diff: (root, path, mode) => Git.diff(root, path, mode, lease()),
    branches: async (root) => {
      selectRoot(root);
      const current = generation;
      const id = lease();
      const result = await Git.branches(root, revision, id);
      const branches: GitBranch[] = [];
      let page = result;
      while (true) {
        if (disposed || current !== generation) throw new Error("Git query cancelled.");
        if (page.error) throw new Error(page.error);
        branches.push(...page.branches.map(branch => ({ ...branch, kind: branch.kind as GitBranch["kind"] })));
        if (result.unchanged || branches.length >= result.total) break;
        if (!page.branches.length) throw new Error("Branch snapshot incomplete. Refresh the branch list.");
        page = await Git.branchPage(result.revision, branches.length, id);
      }
      revision = result.revision;
      return { branches, revision: result.revision, unchanged: result.unchanged, error: null };
    },
    worktrees: (root) => Git.worktrees(root, lease()),
    log: (root, scope, limit, skip) => Git.log(root, scope, limit, skip, lease()),
    resumeHistory: (root, scope, count) => Git.resumeHistory(root, scope, count, lease()),
    matchRefs: (query) => Git.matchRefs(query, lease()),
    commit: (root, oid) => Git.commit(root, oid, lease()),
    fetch: (root) => GitOps.fetch(root),
    pull: (root) => GitOps.pull(root),
    push: (root) => GitOps.push(root),
    checkout: (root, branch) => GitOps.checkout(root, branch),
    createBranch: (root, name, start, checkout) => GitOps.createBranch(root, name, start, checkout),
    deleteBranch: (root, name) => GitOps.deleteBranch(root, name),
    addWorktree: (root, path, branch) => GitOps.addWorktree(root, path, branch),
    removeWorktree: (root, path) => GitOps.removeWorktree(root, path),
    lockWorktree: (root, path) => GitOps.lockWorktree(root, path),
    unlockWorktree: (root, path) => GitOps.unlockWorktree(root, path),
    pruneWorktrees: (root) => GitOps.pruneWorktrees(root),
  };
}
export const nativeGit = createNativeGit();
