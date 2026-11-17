import type { Workspace } from "../workspace/state";
import type { GraphRow } from "./commit-graph";
import type { GitBranch, GitCommit, GitCommitDetail, GitDiffMode, GitRepoStatus, GitRepository, GitTransport, GitWorktree } from "./git-types";
import { gitOperation } from "./operation-gate";

export interface HistoryPreferences {
  scope: string; query: string; searchOpen: boolean; selectedOid: string | null; selectedFile: string | null;
  selectedBranch: string | null; selectedWorktree: string | null; expanded: string[];
  railTab: "branches" | "worktrees"; scrollOffset: number;
}
export interface HistoryCache extends HistoryPreferences {
  version: number; status: GitRepoStatus | null; branches: GitBranch[]; worktrees: GitWorktree[];
  commits: GitCommit[]; graphRows: GraphRow[] | null; graphMaxLanes: number; graphClipped: boolean;
  hasMore: boolean; logTruncated: boolean; detail: GitCommitDetail | null;
  refsKey: string; branchVersion: string; treeVersion: string;
}
export interface ChangesPreferences {
  selected: string | null; expanded: string[]; folders: string[]; scope: GitDiffMode; layout: "unified" | "split";
}
export interface RepositorySession {
  root: string; group: string; resourceKey: string; version: number;
  git: GitTransport; history?: HistoryCache; historyPreferences?: HistoryPreferences; changesPreferences?: ChangesPreferences;
  saveHistory(value: HistoryCache): void;
}
interface Entry extends RepositorySession { base: GitTransport | null; used: number }
const stores = new WeakMap<Workspace, ReturnType<typeof createStore>>();

function createStore(workspace: Workspace) {
  const entries = new Map<string, Entry>();
  let clock = 0;
  function evict(entry: Entry) {
    entry.base?.dispose?.(); entry.base = null; entry.history = undefined; entry.version++;
  }
  workspace.registerRepositoryResources(() => {
    for (const entry of entries.values()) evict(entry);
    entries.clear();
  }, groups => {
    for (const [key, entry] of entries) if (!groups.has(entry.group)) { evict(entry); entries.delete(key); }
  });
  return (group: string, repository: GitRepository, source: GitTransport): RepositorySession => {
    const key = `${group}\0${repository.root}`;
    let entry = entries.get(key);
    if (!entry) {
      entry = { root: repository.root, group, resourceKey: repository.commonDir || repository.root, version: 0,
        git: source, base: null, used: 0,
        saveHistory(value) {
          const { scope, query, searchOpen, selectedOid, selectedFile, selectedBranch, selectedWorktree, expanded, railTab, scrollOffset } = value;
          this.historyPreferences = { scope, query, searchOpen, selectedOid, selectedFile, selectedBranch, selectedWorktree, expanded, railTab, scrollOffset };
          if (value.version === this.version && this.base && value.commits.length <= 5000 && value.branches.length <= 32000) this.history = value;
          else {
            this.history = undefined;
            if (value.version === this.version && this.base) { this.base.dispose?.(); this.base = null; this.version++; }
          }
        },
      };
      entries.set(key, entry);
    }
    entry.used = ++clock;
    entry.resourceKey = repository.commonDir || repository.root;
    if (!entry.base) {
      entry.version++;
      const base = source.fork?.() ?? source;
      entry.base = base;
      const target = entry;
      const mutate = <T>(run: () => Promise<T>) => gitOperation(target.resourceKey, run);
      const git: GitTransport = { ...base,
        status: async root => {
          const status = await base.status(root);
          if (status.repo && !status.error) workspace.updateRepositorySummary(root, {
            branch: status.branch ?? "detached", changes: status.changes.length,
            conflicts: status.changes.filter(change => change.conflicted).length,
          });
          return status;
        },
        fetch: root => mutate(() => base.fetch(root)), pull: root => mutate(() => base.pull(root)), push: root => mutate(() => base.push(root)),
        checkout: (root, branch) => mutate(() => base.checkout(root, branch)),
        createBranch: (root, name, start, checkout) => mutate(() => base.createBranch(root, name, start, checkout)),
        deleteBranch: (root, name) => mutate(() => base.deleteBranch(root, name)),
        addWorktree: (root, path, branch) => mutate(() => base.addWorktree(root, path, branch)),
        removeWorktree: (root, path) => mutate(() => base.removeWorktree(root, path)),
        lockWorktree: (root, path) => mutate(() => base.lockWorktree(root, path)),
        unlockWorktree: (root, path) => mutate(() => base.unlockWorktree(root, path)),
        pruneWorktrees: root => mutate(() => base.pruneWorktrees(root)),
      };
      // A view's disposal cancels reads; the group cache retains the Rust traversal lease.
      git.fork = () => ({ ...git, fork: undefined, dispose: () => base.cancel?.() });
      entry.git = git;
    }
    const retained = [...entries.values()].filter(item => item.base && item !== entry).sort((a, b) => b.used - a.used);
    for (const older of retained.slice(2)) evict(older);
    // Keep a bounded number of lightweight preferences as well.
    if (entries.size > 128) {
      for (const [oldKey, old] of entries) if (!old.base && entries.size > 128) entries.delete(oldKey);
    }
    return entry;
  };
}
export function repositorySession(workspace: Workspace, group: string, repository: GitRepository, source: GitTransport) {
  let store = stores.get(workspace);
  if (!store) { store = createStore(workspace); stores.set(workspace, store); }
  return store(group, repository, source);
}
