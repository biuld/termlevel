import { batch, cleanup, effect, lifecycle, untracked } from "nsh";
import type { GitBranches, GitLog, GitRepoStatus, GitWorktrees } from "../git-types";
import { LOG_PAGE, errorMessage } from "./common";
import type { HistoryState } from "./state";

export function createHistoryData(state: HistoryState) {
  const {
    props, cached, sessionVersion, scrollOffset, searchOpen, railTab, expanded, cwd, status, branches, worktrees, commits, graphRows, graphMaxLanes, graphClipped,
    hasMore, logTruncated, selectedOid, detail, selectedFile, scope, query, refMatches, selectedBranch,
    selectedWorktree, loading, logLoading, loadingMore, detailLoading, error, logError, busy, epoch,
  } = state;
  let disposed = false;
  let syncing = false;
  let loadGeneration = 0;
  let logGeneration = 0;
  let detailGeneration = 0;
  let lastPath = cwd();
  let firstLoad = true;
  let lastRefs = cached?.refsKey ?? "";
  let lastBranches = cached?.branchVersion ?? "";
  let lastWorktrees = cached?.treeVersion ?? "";
  let searchGeneration = 0;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  const alive = (generation: number) => !disposed && generation === loadGeneration;
  cleanup(() => { disposed = true; loadGeneration++; logGeneration++; detailGeneration++; searchGeneration++; if (searchTimer !== undefined) clearTimeout(searchTimer); });

  cleanup(() => untracked(() => {
    props.session?.saveHistory({
      version: sessionVersion, status: status(), branches: branches(), worktrees: worktrees(), commits: commits(),
      graphRows: graphRows(), graphMaxLanes: graphMaxLanes(), graphClipped: graphClipped(), hasMore: hasMore(),
      logTruncated: logTruncated(), detail: detail(), scope: scope(), query: query(), searchOpen: searchOpen(),
      selectedOid: selectedOid(), selectedFile: selectedFile(), selectedBranch: selectedBranch(), selectedWorktree: selectedWorktree(),
      expanded: expanded(), railTab: railTab(), scrollOffset: scrollOffset(),
      refsKey: lastRefs, branchVersion: lastBranches, treeVersion: lastWorktrees,
    });
  }));

  function applyLog(log: GitLog, append: boolean) {
    if (log.error) throw new Error(log.error);
    const previous = untracked(() => selectedOid());
    const all = append ? [...untracked(() => commits()), ...log.commits] : log.commits;
    const rows = log.graph?.length === log.commits.length
      ? log.commits.map((commit, index) => ({ commit, ...log.graph![index] })) : null;
    batch(() => {
      graphRows.set(rows ? append ? [...untracked(() => graphRows()) ?? [], ...rows] : rows : null);
      graphMaxLanes.set(log.graphMaxLanes ?? 0);
      graphClipped.set(log.graphClipped ?? false);
      commits.set(all);
      hasMore.set(log.hasMore);
      logTruncated.set(log.truncated);
      logError.set("");
      if (!append) selectedOid.set(previous && all.some(commit => commit.oid === previous) ? previous : all[0]?.oid ?? null);
    });
  }

  function applyRefs(next: GitRepoStatus, refs: GitBranches, trees: GitWorktrees) {
    if (next.error || refs.error || trees.error) throw new Error(next.error || refs.error || trees.error || "Git refresh failed.");
    // Native reads provide a small revision token. Only injected transports need this fallback.
    const branchVersion = refs.revision ?? JSON.stringify(refs.branches);
    const key = JSON.stringify([next.branch, next.detached, branchVersion]);
    const treeVersion = JSON.stringify(trees.worktrees);
    const changed = key !== lastRefs;
    batch(() => {
      status.set(next);
      if (!refs.unchanged && branchVersion !== lastBranches) branches.set(refs.branches);
      if (treeVersion !== lastWorktrees) worktrees.set(trees.worktrees);
      error.set("");
    });
    lastBranches = branchVersion;
    lastWorktrees = treeVersion;
    lastRefs = key;
    return changed;
  }

  async function reloadLog(nextScope = scope()) {
    if (disposed) return;
    const generation = ++logGeneration;
    props.git.cancel?.("log");
    logLoading.set(true);
    loadingMore.set(false);
    logError.set("");
    try {
      const log = await props.git.log(cwd(), nextScope, LOG_PAGE, 0);
      if (disposed || generation !== logGeneration) return;
      applyLog(log, false);
    } catch (next) {
      if (disposed || generation !== logGeneration) return;
      batch(() => { commits.set([]); graphRows.set(null); hasMore.set(false); selectedOid.set(null); logError.set(errorMessage(next)); });
    } finally {
      if (!disposed && generation === logGeneration) logLoading.set(false);
    }
  }

  async function load() {
    if (disposed) return;
    const path = cwd();
    const restoring = firstLoad && !!cached; firstLoad = false;
    const generation = ++loadGeneration;
    // Invalidate pending log/detail responses before waiting for the new repository status.
    logGeneration++;
    detailGeneration++;
    props.git.cancel?.();
    if (path !== lastPath) {
      lastPath = path;
      lastRefs = lastBranches = lastWorktrees = "";
      batch(() => {
        scope.set(""); selectedBranch.set(null); selectedWorktree.set(null);
        selectedOid.set(null); detail.set(null); selectedFile.set(null); query.set("");
        branches.set([]); worktrees.set([]); commits.set([]); graphRows.set(null); hasMore.set(false);
      });
    }
    loading.set(true);
    loadingMore.set(false);
    logLoading.set(false);
    detailLoading.set(false);
    error.set("");
    try {
      const next = await props.git.status(path);
      if (!alive(generation)) return;
      if (next.error) throw new Error(next.error);
      status.set(next);
      if (!next.repo) {
        batch(() => {
          branches.set([]); worktrees.set([]); commits.set([]); graphRows.set(null);
          hasMore.set(false); logError.set(""); selectedOid.set(null); detail.set(null); selectedFile.set(null);
        });
        return;
      }
      const [refs, trees] = await Promise.all([props.git.branches(path), props.git.worktrees(path)]);
      if (!alive(generation)) return;
      const changed = applyRefs(next, refs, trees);
      const current = untracked(() => scope());
      const valid = current && branches().some(branch => branch.fullName === current) ? current : "";
      if (valid !== current) batch(() => { scope.set(valid); selectedBranch.set(valid || null); });
      const previousOid = untracked(() => selectedOid());
      const resume = restoring && !changed && valid === current && commits().length > 0
        && (!props.git.resumeHistory || await props.git.resumeHistory(path, valid, commits().length));
      if (!alive(generation)) return;
      if (!resume) await reloadLog(valid);
      // A refresh may retain the selected SHA; explicitly reload its detail as well.
      if (alive(generation) && previousOid === selectedOid()) void loadDetail();
    } catch (next) {
      if (alive(generation)) error.set(errorMessage(next));
    } finally {
      if (alive(generation)) loading.set(false);
    }
  }

  async function syncRefs() {
    if (props.preview || busy() || loading() || logLoading() || loadingMore() || syncing || disposed) return;
    syncing = true;
    const path = cwd();
    const generation = loadGeneration;
    try {
      const [next, refs, trees] = await Promise.all([props.git.status(path), props.git.branches(path), props.git.worktrees(path)]);
      if (!alive(generation) || path !== cwd()) return;
      const changed = applyRefs(next, refs, trees);
      const current = untracked(() => scope());
      if (current && !branches().some(branch => branch.fullName === current)) {
        batch(() => { scope.set(""); selectedBranch.set(null); });
        await reloadLog("");
      } else if (changed) await reloadLog(current);
    } catch (next) {
      if (alive(generation)) error.set(errorMessage(next));
    } finally { syncing = false; }
  }

  async function loadMore() {
    if (loading() || logLoading() || loadingMore() || !hasMore() || disposed) return;
    const generation = logGeneration;
    loadingMore.set(true);
    try {
      const log = await props.git.log(cwd(), scope(), LOG_PAGE, untracked(() => commits().length));
      if (disposed || generation !== logGeneration) return;
      applyLog(log, true);
    } catch (next) {
      if (!disposed && generation === logGeneration) logError.set(errorMessage(next));
    } finally {
      if (!disposed && generation === logGeneration) loadingMore.set(false);
    }
  }

  async function loadDetail() {
    if (disposed) return;
    const oid = selectedOid();
    const path = cwd();
    const generation = ++detailGeneration;
    props.git.cancel?.("commit");
    if (!oid) { detail.set(null); selectedFile.set(null); detailLoading.set(false); return; }
    const previousFile = detail()?.commit.oid === oid ? selectedFile() : null;
    detailLoading.set(true);
    try {
      const next = await props.git.commit(path, oid);
      if (disposed || generation !== detailGeneration) return;
      detail.set(next);
      selectedFile.set(previousFile && next.files.some(file => file.path === previousFile) ? previousFile : next.files[0]?.path ?? null);
    } catch {
      if (disposed || generation !== detailGeneration) return;
      detail.set(null); selectedFile.set(null);
    } finally {
      if (!disposed && generation === detailGeneration) detailLoading.set(false);
    }
  }

  effect(() => {
    const text = query().trim();
    commits();
    const generation = ++searchGeneration;
    if (searchTimer !== undefined) clearTimeout(searchTimer);
    props.git.cancel?.("search");
    refMatches.set(new Set());
    if (text && props.git.matchRefs) searchTimer = setTimeout(() => {
      void props.git.matchRefs!(text).then(oids => {
        if (!disposed && generation === searchGeneration) refMatches.set(new Set(oids));
      }).catch(() => {});
    }, 100);
  });

  function refresh() { props.rescan?.(); epoch.update(value => value + 1); }
  effect(() => { cwd(); epoch(); untracked(() => { void load(); }); });
  effect(() => { if (props.refreshTick) { props.refreshTick(); untracked(() => { void syncRefs(); }); } });
  effect(() => { selectedOid(); cwd(); untracked(() => { void loadDetail(); }); });
  if (!props.preview && !props.refreshTick) {
    const timer = setInterval(() => { if (lifecycle() === "active") void syncRefs(); }, 5000);
    cleanup(() => clearInterval(timer));
  }
  return { reloadLog, load, loadMore, refresh };
}
export type HistoryData = ReturnType<typeof createHistoryData>;
