import { batch } from "nsh";

import { errorMessage } from "./common";
import type { HistoryState } from "./state";
import type { HistoryData } from "./data";

export function createHistoryActions(state: HistoryState, data: HistoryData) {
  const {
    props, store, groupId, cwd, status, branches, worktrees, scope, railTab, selectedBranch,
    selectedWorktree, opError, networkAction, busy, branchDialog, deleteDialog, worktreeDialog,
    removeDialog, newBranchName, newBranchCheckout, branchStart, newWorktreePath,
    newWorktreeBranch, removeTarget, deleteTarget, repo, selectedBranchInfo, canCheckout,
    canDeleteBranch, selectedWorktreeInfo,
  } = state;
  const { reloadLog, load } = data;
  async function updateRemote(action: "fetch" | "pull") {
    if (busy() || props.preview || !repo()) return;
    const path = cwd();
    const owner = groupId();
    const ownerTitle = store.groupTitle(owner);
    busy.set(true);
    networkAction.set(action);
    opError.set("");
    let failure = "";
    try {
      await props.git[action](path);
    } catch (next) {
      failure = errorMessage(next);
    } finally {
      props.notify(failure ? "error" : "info", `${action === "fetch" ? "Fetch" : "Pull"} ${failure ? "failed" : "complete"}`, failure ? `${ownerTitle} · ${failure}` : ownerTitle);
      // Pull may fetch refs even when the fast-forward fails.
      if (groupId() === owner && cwd() === path) {
        await load();
      }
      networkAction.set(null);
      busy.set(false);
    }
  }

  function selectScope(id: string) {
    if (id === "scope:all") {
      if (!scope()) return;
      batch(() => {
        scope.set("");
        selectedBranch.set(null);
      });
      void reloadLog("");
      return;
    }
    if (!id.startsWith("branch:")) return;
    const full = id.slice("branch:".length);
    if (scope() === full) return;
    batch(() => {
      scope.set(full);
      selectedBranch.set(full);
    });
    void reloadLog(full);
  }

  function checkoutBranch(fullName: string) {
    const branch = branches().find((item) => item.fullName === fullName);
    if (!branch || branch.current || branch.kind === "tag" || props.preview) return;
    const holder = worktrees().find((worktree) => worktree.branch === branch.name);
    if (holder && !holder.current) return;
    void runOp(async () => {
      await props.git.checkout(cwd(), branch.name);
      await load();
    });
  }

  function checkoutSelected() {
    const branch = selectedBranchInfo();
    if (branch && canCheckout()) checkoutBranch(branch.fullName);
  }

  function openBranchDialog(start = "") {
    if (props.preview) return;
    branchStart.set(start);
    newBranchName.set("");
    newBranchCheckout.set(true);
    opError.set("");
    branchDialog.set(true);
  }

  function openDeleteDialog() {
    const branch = selectedBranchInfo();
    if (!branch || !canDeleteBranch() || props.preview) return;
    deleteTarget.set(branch.name);
    opError.set("");
    deleteDialog.set(true);
  }

  function deleteBranch() {
    const branch = branches().find((item) => item.kind === "local" && item.name === deleteTarget());
    if (!branch) return;
    void runOp(async () => {
      await props.git.deleteBranch(cwd(), branch.name);
      deleteDialog.set(false);
      if (scope() === branch.fullName) {
        batch(() => {
          scope.set("");
          selectedBranch.set(null);
        });
      }
      await load();
    });
  }

  function openWorktreeDialog() {
    if (props.preview) return;
    const selected = selectedBranchInfo();
    const checkedOut = (name: string) => worktrees().some((worktree) => worktree.branch === name);
    let branch = selected?.kind === "local" && !checkedOut(selected.name) ? selected : undefined;
    if (!branch) {
      branch = branches().find((item) => item.kind === "local" && !checkedOut(item.name));
    }
    if (!branch) {
      opError.set(selected?.kind === "local" && checkedOut(selected.name)
        ? `${selected.name} is already checked out in a worktree.`
        : "Every local branch is checked out. Create a branch without checkout first.");
      return;
    }
    const root = status()?.root ?? cwd();
    newWorktreeBranch.set(branch.name);
    newWorktreePath.set(`${root.replace(/[\\/]+$/, "")}-wt/${branch.name.replace(/[\\/]+/g, "-")}`);
    opError.set("");
    worktreeDialog.set(true);
  }

  function openRemoveDialog() {
    const worktree = selectedWorktreeInfo();
    if (!worktree || props.preview) return;
    removeTarget.set(worktree.path);
    opError.set("");
    removeDialog.set(true);
  }

  function openWorktree(path: string) {
    if (props.preview) return;
    store.addGroup(path);
  }

  function toggleWorktreeLock() {
    const worktree = selectedWorktreeInfo();
    if (!worktree || worktree.main || props.preview) return;
    void runOp(async () => {
      if (worktree.locked) await props.git.unlockWorktree(cwd(), worktree.path);
      else await props.git.lockWorktree(cwd(), worktree.path);
      await load();
    });
  }

  async function runOp(action: () => Promise<void>) {
    if (busy()) return;
    busy.set(true);
    opError.set("");
    try {
      await action();
    } catch (next) {
      opError.set(errorMessage(next));
    } finally {
      busy.set(false);
    }
  }

  function createBranch() {
    const name = newBranchName().trim();
    if (!name) {
      opError.set("Enter a branch name.");
      return;
    }
    void runOp(async () => {
      await props.git.createBranch(cwd(), name, branchStart() || scope() || "HEAD", newBranchCheckout());
      branchDialog.set(false);
      batch(() => {
        scope.set(`refs/heads/${name}`);
        selectedBranch.set(`refs/heads/${name}`);
      });
      await load();
    });
  }

  function createWorktree() {
    const path = newWorktreePath().trim();
    if (!path) {
      opError.set("Enter a folder for the worktree.");
      return;
    }
    void runOp(async () => {
      const created = await props.git.addWorktree(cwd(), path, newWorktreeBranch());
      worktreeDialog.set(false);
      batch(() => {
        railTab.set("worktrees");
        selectedWorktree.set(created);
      });
      await load();
      selectedWorktree.set(created);
    });
  }

  function removeWorktree() {
    void runOp(async () => {
      await props.git.removeWorktree(cwd(), removeTarget());
      removeDialog.set(false);
      selectedWorktree.set(null);
      await load();
    });
  }

  return {
    updateRemote, selectScope, checkoutBranch, checkoutSelected, openBranchDialog,
    openDeleteDialog, deleteBranch, openWorktreeDialog, openRemoveDialog, openWorktree,
    toggleWorktreeLock, runOp, createBranch, createWorktree, removeWorktree,
  };
}

export type HistoryActions = ReturnType<typeof createHistoryActions>;
