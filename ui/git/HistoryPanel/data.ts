import { batch, cleanup, effect, lifecycle, untracked } from "nsh";

import type { GitLog } from "../git-types";
import { LOG_PAGE, errorMessage } from "./common";
import type { HistoryState } from "./state";

export function createHistoryData(state: HistoryState) {
  const {
    props, cwd, status, branches, worktrees, commits, hasMore, logTruncated, selectedOid, detail,
    selectedFile, scope, query, selectedBranch, selectedWorktree, loading, logLoading, loadingMore,
    detailLoading, error, logError, busy, epoch,
  } = state;
  let loadGeneration = 0;
  let logGeneration = 0;
  let detailGeneration = 0;
  let lastPath = "";
  function applyLog(log: GitLog, append: boolean) {
    const previous = untracked(() => selectedOid());
    const all = append ? [...untracked(() => commits()), ...log.commits] : log.commits;
    batch(() => {
      commits.set(all);
      hasMore.set(log.hasMore);
      logTruncated.set(log.truncated);
      logError.set(log.error ?? "");
      if (!append) {
        const keep = previous && all.some((commit) => commit.oid === previous) ? previous : all[0]?.oid ?? null;
        selectedOid.set(keep);
      }
    });
  }

  async function reloadLog(nextScope = scope()) {
    const generation = ++logGeneration;
    logLoading.set(true);
    logError.set("");
    try {
      const log = await props.git.log(cwd(), nextScope, LOG_PAGE, 0);
      if (generation !== logGeneration) return;
      applyLog(log, false);
    } catch (next) {
      if (generation !== logGeneration) return;
      commits.set([]);
      logError.set(errorMessage(next));
    } finally {
      if (generation === logGeneration) logLoading.set(false);
    }
  }

  async function load() {
    const path = cwd();
    const generation = ++loadGeneration;
    if (path !== lastPath) {
      lastPath = path;
      batch(() => {
        scope.set("");
        selectedBranch.set(null);
        selectedWorktree.set(null);
        selectedOid.set(null);
        detail.set(null);
        selectedFile.set(null);
        query.set("");
      });
    }
    loading.set(true);
    error.set("");
    try {
      const next = await props.git.status(path);
      if (generation !== loadGeneration) return;
      status.set(next);
      if (!next.repo) {
        batch(() => {
          branches.set([]);
          worktrees.set([]);
          commits.set([]);
          hasMore.set(false);
          logError.set("");
          selectedOid.set(null);
          detail.set(null);
          selectedFile.set(null);
        });
        return;
      }
      const [branchData, worktreeData] = await Promise.all([
        props.git.branches(path),
        props.git.worktrees(path),
      ]);
      if (generation !== loadGeneration) return;
      branches.set(branchData.branches);
      worktrees.set(worktreeData.worktrees);
      if (branchData.error) error.set(branchData.error);
      if (worktreeData.error) error.set(worktreeData.error);
      const current = untracked(() => scope());
      const valid = current && branchData.branches.some((branch) => branch.fullName === current) ? current : "";
      if (valid !== current) {
        scope.set(valid);
        selectedBranch.set(valid || null);
      }
      await reloadLog(valid);
    } catch (next) {
      if (generation !== loadGeneration) return;
      status.set(null);
      error.set(errorMessage(next));
    } finally {
      if (generation === loadGeneration) loading.set(false);
    }
  }

  async function syncRefs() {
    if (props.preview || busy() || loading() || logLoading()) return;
    const path = cwd();
    const generation = loadGeneration;
    try {
      const [nextStatus, branchData, worktreeData] = await Promise.all([
        props.git.status(path),
        props.git.branches(path),
        props.git.worktrees(path),
      ]);
      if (generation !== loadGeneration || path !== cwd()) return;
      const previousRefs = untracked(() => branches().map((branch) => `${branch.fullName}:${branch.target}`).join("|"));
      const nextRefs = branchData.branches.map((branch) => `${branch.fullName}:${branch.target}`).join("|");
      status.set(nextStatus);
      branches.set(branchData.branches);
      worktrees.set(worktreeData.worktrees);
      const current = untracked(() => scope());
      if (current && !branchData.branches.some((branch) => branch.fullName === current)) {
        batch(() => {
          scope.set("");
          selectedBranch.set(null);
        });
        await reloadLog("");
      } else if (previousRefs !== nextRefs) {
        await reloadLog(current);
      }
    } catch {
      // A transient refresh failure keeps the last good snapshot.
    }
  }

  async function loadMore() {
    if (logLoading() || loadingMore() || !hasMore()) return;
    const generation = logGeneration;
    const skip = untracked(() => commits().length);
    loadingMore.set(true);
    try {
      const log = await props.git.log(cwd(), scope(), LOG_PAGE, skip);
      if (generation !== logGeneration) return;
      applyLog(log, true);
    } catch (next) {
      if (generation !== logGeneration) return;
      logError.set(errorMessage(next));
    } finally {
      loadingMore.set(false);
    }
  }

  function refresh() {
    epoch.update((value) => value + 1);
  }

  effect(() => {
    cwd();
    epoch();
    void load();
  });

  if (!props.preview) {
    const timer = setInterval(() => {
      if (lifecycle() === "active") void syncRefs();
    }, 5000);
    cleanup(() => clearInterval(timer));
  }

  effect(() => {
    const oid = selectedOid();
    const path = cwd();
    const generation = ++detailGeneration;
    if (!oid) {
      detail.set(null);
      selectedFile.set(null);
      detailLoading.set(false);
      return;
    }
    detailLoading.set(true);
    void props.git.commit(path, oid).then((next) => {
      if (generation !== detailGeneration) return;
      detail.set(next);
      selectedFile.set(next.files[0]?.path ?? null);
    }).catch(() => {
      if (generation !== detailGeneration) return;
      detail.set(null);
      selectedFile.set(null);
    }).finally(() => {
      if (generation === detailGeneration) detailLoading.set(false);
    });
  });

  return { reloadLog, load, loadMore, refresh };
}

export type HistoryData = ReturnType<typeof createHistoryData>;
