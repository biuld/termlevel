import { Scroll, token, View } from "nsh";
import { Button, Checkbox, Input, ResizablePanel, ResizablePanelGroup, Tree } from "nsh/base";

import { Action, AppDialog, DialogActions, DialogKeys, Hint, IconButton } from "../../shared/controls";

import { Icon } from "../../shared/icons";
import { Type } from "../../shared/type";

import { scopedGit } from "../scoped-git";

import { pathName, RefHint, type HistoryProps } from "./common";
import { createHistoryState } from "./state";
import { createHistoryData } from "./data";
import { createHistoryActions } from "./actions";
import { createHistoryViews } from "./views";

export function HistoryPanel(props: HistoryProps) {
  const state = createHistoryState({ ...props, git: scopedGit(props.git) });
  const data = createHistoryData(state);
  const actions = createHistoryActions(state, data);
  const views = createHistoryViews(state, data, actions);
  const {
    store, cwd, status, worktrees, commits, scope, query, searchOpen, railTab, expanded, loading,
    logLoading, opError, networkAction, busy, branchDialog, deleteDialog, worktreeDialog,
    removeDialog, newBranchName, newBranchCheckout, newWorktreePath, newWorktreeBranch,
    removeTarget, deleteTarget, failure, repo, filtered, selectedWorktreeIndex, selectedBranchInfo,
    selectedBranchWorktree, canCheckout, canDeleteBranch, selectedWorktreeInfo, canRemoveWorktree, scopeLabel, startLabel,
    branchItems, wide,
  } = state;
  const { refresh } = data;
  const {
    updateRemote, selectScope, checkoutBranch, checkoutSelected, openBranchDialog,
    openDeleteDialog, deleteBranch, openWorktreeDialog, openRemoveDialog, openWorktree,
    toggleWorktreeLock, createBranch, createWorktree, removeWorktree,
  } = actions;
  const { renderBranchItem, WorktreeRowView, commitPane, detailWide, detailNarrow } = views;
  return <View testID="history-panel" class="min-h-0 flex-1 flex-col">
    <View class="h-(--height-toolbar) shrink-0 flex-row items-center gap-2 border-b border-border px-3">
      <If when={!(searchOpen() && (store.canvasSize().width || 1200) < 650)}><Icon name="gitBranch" size={14} color={token("icon")} /></If>
      <If when={repo() && !(searchOpen() && (store.canvasSize().width || 1200) < 650)}>
        <View class="min-w-0 flex-1 flex-row items-center gap-2">
        <Type role="body" family="mono" tone="foreground" class={scope() ? "min-w-0 flex-1" : "shrink-0"} lines={1} overflow="ellipsis-middle" tooltip={<RefHint label={scopeLabel()} />}>{scopeLabel()}</Type>
        <If when={!!scope()}>
          <IconButton icon="close" label="Show all branches" testID="history-clear-scope"
            onPress={() => selectScope("scope:all")} />
        </If>
        <View class="h-3 w-px bg-border" />
        <If when={status()?.detached}>
          <Type role="caption" tone="subtle-foreground">{`HEAD@${status()?.branch ?? ""}`}</Type>
        </If>
        <If when={!status()?.detached && !scope()}>
          <Type role="caption" tone="subtle-foreground" class="min-w-0 flex-1" lines={1} overflow="ellipsis-middle" tooltip={<RefHint label={status()?.branch ?? ""} />}>{status()?.branch ?? ""}</Type>
        </If>
        <If when={(status()?.ahead ?? 0) > 0 || (status()?.behind ?? 0) > 0}>
          <Type role="caption" tone="subtle-foreground">{`↑${status()?.ahead ?? 0} ↓${status()?.behind ?? 0}`}</Type>
        </If>
        <If when={!!status()?.operation}>
          <Type role="caption" tone="subtle-foreground">{status()!.operation!.toUpperCase()}</Type>
        </If>
        <If when={props.preview}>
          <Type role="caption" tone="subtle-foreground" class="ml-1"
            tooltip={<Hint>History is deterministic in preview mode; Git commands do not run.</Hint>}>Preview</Type>
        </If>
        </View>
      </If>
      <View class="ml-auto shrink-0 flex-row items-center gap-1.5">
        <IconButton icon={networkAction() === "fetch" ? "refresh" : "gitFetch"} label={networkAction() === "fetch" ? "Fetching…" : "Fetch from remote"}
          testID="history-fetch" disabled={props.preview || busy() || loading() || !repo()} onPress={() => { void updateRemote("fetch"); }} />
        <IconButton icon={networkAction() === "pull" ? "refresh" : "gitPull"} label={networkAction() === "pull" ? "Pulling…" : "Pull current branch (fast-forward only)"}
          testID="history-pull" disabled={props.preview || busy() || loading() || !repo()} onPress={() => { void updateRemote("pull"); }} />
        <IconButton icon={networkAction() === "push" ? "refresh" : "gitPush"} label={networkAction() === "push" ? "Pushing…" : "Push current branch to upstream"}
          testID="history-push" disabled={props.preview || busy() || loading() || !repo()} onPress={() => { void updateRemote("push"); }} />
        <If when={!searchOpen()}>
          <IconButton icon="search" label="Search loaded commits" testID="history-search-open" onPress={() => searchOpen.set(true)} />
        </If>
        <If when={searchOpen()}>
          <DialogKeys dismiss={() => { searchOpen.set(false); query.set(""); }} />
          <View class="h-(--height-control) flex-row items-center gap-1 rounded-(--radius-control) border border-sidebar-border bg-search-surface px-1.5">
            <Icon name="search" size={12} color={token("muted-foreground")} />
            <Input testID="history-search" value={query()} onChange={({ value }) => query.set(value)} autoFocus
              placeholder="Search commits" accessibilityLabel="Search loaded commits"
              class="h-full text-(length:--text-body) text-foreground"
              style={{ width: (store.canvasSize().width || 1200) < 700 ? 90 : 170 }} />
            <If when={!!query()}><Type role="caption" tone="muted-foreground" tooltip={<Hint>Matches in loaded commits</Hint>}>{`${filtered().length}/${commits().length}`}</Type></If>
            <IconButton icon="close" label="Close commit search" testID="history-search-close" onPress={() => { searchOpen.set(false); query.set(""); }} />
          </View>
        </If>
        <View class="relative">
          <IconButton icon="refresh" label={logLoading() || loading() ? "Refreshing history…" : "Refresh history"} testID="history-refresh" onPress={refresh} />
          <If when={logLoading() || loading()}><View testID="history-refreshing" class="absolute right-0 top-0 h-1.5 w-1.5 rounded-full bg-primary" style={{ pointerEvents: "none" }} /></If>
        </View>
      </View>
    </View>

    <If when={!status() && loading()}>
      <View class="flex-1 items-center justify-center gap-2">
        <Type role="heading">Loading history…</Type>
        <Type role="body" tone="muted-foreground">Reading branches, worktrees and commits.</Type>
      </View>
    </If>

    <If when={!!failure()}>
      <View class="flex-1 items-center justify-center gap-3 px-6">
        <Icon name="alert" size={18} color={token("destructive")} />
        <Type role="heading">Git needs attention.</Type>
        <Type role="body" tone="muted-foreground" class="text-center">{failure()}</Type>
        <IconButton icon="refresh" label="Retry Git" onPress={refresh} />
      </View>
    </If>

    <If when={!failure() && status() && !repo()}>
      <View class="flex-1 items-center justify-center gap-3 px-6">
        <Icon name="folder" size={18} />
        <Type role="heading">No Git repository here.</Type>
        <Type role="body" tone="muted-foreground" class="text-center">This folder is not inside a Git work tree.</Type>
        <Type role="body" family="mono" tone="subtle-foreground" lines={1}>{cwd()}</Type>
      </View>
    </If>

    <If when={!failure() && repo()}>
      <If when={!!opError()}>
        <View class="shrink-0 border-b border-border bg-notification-error-muted px-3 py-1">
          <Type role="caption" tone="foreground">{opError()}</Type>
        </View>
      </If>
      <ResizablePanelGroup class="min-h-0 flex-1"
        handleClass="bg-transparent group-hover:bg-sidebar-border group-active:bg-ring"
        handleAccessibilityLabel="Resize history panes">
        <ResizablePanel size={wide() ? 252 : 220} minSize={180} maxSize={340}>
          <View class="h-full min-h-0 flex-col border-r border-border">
            <View class="h-(--height-toolbar) shrink-0 flex-row items-center gap-1 border-b border-border px-2">
              <View class="flex-row rounded-(--radius-tab) bg-chrome p-0.5">
                <Button testID="history-tab-branches" selected={railTab() === "branches"}
                  onPress={() => railTab.set("branches")}
                  class="h-(--height-control) items-center justify-center rounded-(--radius-control) px-2 focus-visible:ring-2 ring-sidebar-ring"
                  classList={{ "bg-sidebar": railTab() === "branches" }}>
                  <Type role="label" tone={railTab() === "branches" ? "foreground" : "muted-foreground"}>Branches</Type>
                </Button>
                <Button testID="history-tab-worktrees" selected={railTab() === "worktrees"}
                  onPress={() => railTab.set("worktrees")}
                  class="h-(--height-control) items-center justify-center rounded-(--radius-control) px-2 focus-visible:ring-2 ring-sidebar-ring"
                  classList={{ "bg-sidebar": railTab() === "worktrees" }}>
                  <Type role="label" tone={railTab() === "worktrees" ? "foreground" : "muted-foreground"}>Worktrees</Type>
                </Button>
              </View>
              <View class="ml-auto flex-row items-center gap-0.5">
                <If when={railTab() === "branches"}>
                  <IconButton icon="plus" label="New branch" testID="history-new-branch"
                    disabled={props.preview} onPress={() => openBranchDialog()} />
                </If>
                <If when={railTab() === "worktrees"}>
                  <IconButton icon="plus" label="New worktree" testID="history-new-worktree"
                    disabled={props.preview} onPress={openWorktreeDialog} />
                </If>
              </View>
            </View>

            <If when={railTab() === "branches"}>
              <Tree class="min-h-0 flex-1" listClass="py-1" accessibilityLabel="Branches and tags"
                items={branchItems()} expanded={expanded()} selected={scope() ? `branch:${scope()}` : "scope:all"}
                itemClass="mx-1 rounded-(--radius-tab) hover:bg-tab-hover selected:bg-tab-selected"
                renderItem={renderBranchItem}
                onExpandedChange={({ expanded: next }) => expanded.set(next)}
                onSelectedChange={({ selected }) => selectScope(selected)}
                onAction={({ id }) => { if (id.startsWith("branch:")) checkoutBranch(id.slice("branch:".length)); }} />
              <View testID="history-branch-actions" class="shrink-0 border-t border-border px-2 py-2">
                <View class="flex-row items-center gap-1">
                  <IconButton icon="gitCheckout" label={selectedBranchWorktree() && !selectedBranchInfo()?.current
                    ? `Checked out in ${pathName(selectedBranchWorktree()!.path)}`
                    : (status()?.changes.length ?? 0) > 0 ? "Checkout branch (working tree has changes)" : "Checkout branch"} testID="history-checkout"
                    disabled={props.preview || busy() || !canCheckout()} onPress={checkoutSelected} />
                  <IconButton icon="gitWorktree" label="New worktree from this branch" testID="history-branch-worktree"
                    disabled={props.preview || busy() || !selectedBranchInfo() || selectedBranchInfo()?.kind !== "local"}
                    onPress={openWorktreeDialog} />
                  <IconButton icon="trash" label="Delete branch" testID="history-delete-branch"
                    disabled={props.preview || busy() || !canDeleteBranch()} onPress={openDeleteDialog} />
                </View>
              </View>
            </If>

            <If when={railTab() === "worktrees"}>
              <If when={worktrees().length > 0}>
                <Scroll testID="history-worktree-list" class="min-h-0 flex-1" contentClass="flex-col py-1">
                  <View class="relative flex-col">
                    <Each of={worktrees()} key={(worktree) => worktree.path}>
                      {(worktree) => <WorktreeRowView worktree={worktree()} />}
                    </Each>
                    <If when={selectedWorktreeIndex() >= 0}>
                      <View class="absolute left-0 right-0 bg-primary/20"
                        style={{
                          top: selectedWorktreeIndex() * 46,
                          height: 46,
                          pointerEvents: "none",
                          zIndex: 1,
                        }} />
                    </If>
                  </View>
                </Scroll>
              </If>
              <If when={worktrees().length === 0}>
                <View class="flex-1 items-center justify-center gap-2 px-4">
                  <Icon name="gitWorktree" size={18} />
                  <Type role="body" tone="muted-foreground" class="text-center">No worktrees for this repository.</Type>
                </View>
              </If>
              <View class="shrink-0 border-t border-border px-2 py-2">
                <View class="flex-row items-center gap-1">
                  <Button testID="history-worktree-open" disabled={props.preview || !selectedWorktreeInfo()}
                    onPress={() => { const worktree = selectedWorktreeInfo(); if (worktree) openWorktree(worktree.path); }}
                    class="h-(--height-action) flex-1 items-center justify-center rounded-(--radius-control) px-2 focus-visible:ring-2 ring-ring"
                    classList={{ "bg-tab-selected": !!selectedWorktreeInfo(), "opacity-40": !selectedWorktreeInfo() }}>
                    <Type role="label">Open in new group</Type>
                  </Button>
                  <IconButton icon="lock" label={selectedWorktreeInfo()?.locked ? "Unlock worktree" : "Lock worktree"}
                    disabled={props.preview || !selectedWorktreeInfo() || !!selectedWorktreeInfo()!.main}
                    onPress={toggleWorktreeLock} />
                  <IconButton icon="close" label="Remove worktree" testID="history-worktree-remove"
                    disabled={props.preview || !canRemoveWorktree()} onPress={openRemoveDialog} />
                </View>
                <If when={!!selectedWorktreeInfo()}>
                  <Type role="caption" tone="subtle-foreground" class="mt-1 px-1" lines={1}>
                    {selectedWorktreeInfo()!.path}
                  </Type>
                </If>
              </View>
            </If>
          </View>
        </ResizablePanel>

        <ResizablePanel minSize={320}>
          <If when={wide()}>{commitPane()}</If>
          <If when={!wide()}>
            <ResizablePanelGroup axis="vertical" class="h-full min-h-0"
              handleClass="bg-transparent group-hover:bg-sidebar-border group-active:bg-ring"
              handleAccessibilityLabel="Resize commits and commit details">
              <ResizablePanel minSize={180}>{commitPane()}</ResizablePanel>
              {/* NSH 0.5.0 does not clamp the growing panel's max during a drag. */}
              <ResizablePanel size={300} minSize={170}>{detailNarrow()}</ResizablePanel>
            </ResizablePanelGroup>
          </If>
        </ResizablePanel>

        {/* Keep the growing side uncapped so the handle cannot pass its edge. */}
        <ResizablePanel size={390} minSize={300} visible={wide()}>
          {detailWide()}
        </ResizablePanel>
      </ResizablePanelGroup>
    </If>

    <AppDialog open={branchDialog()} label="New branch" title="New branch" frame="w-[380px]"
      dismiss={() => branchDialog.set(false)} closeLabel="Cancel new branch">
      <Type role="caption">{`Created from ${startLabel()}.`}</Type>
      <Input testID="history-new-branch-name" value={newBranchName()}
        onChange={({ value }) => newBranchName.set(value)} placeholder="Branch name"
        accessibilityLabel="Branch name" autoFocus
        class="h-(--height-input) rounded-(--radius-control) border border-sidebar-border bg-sidebar px-3 text-(length:--text-body)" />
      <Checkbox testID="history-new-branch-checkout" checked={newBranchCheckout()}
        onChange={({ checked }) => newBranchCheckout.set(checked)}
        accessibilityLabel="Checkout the new branch"
        class="flex-row items-center gap-2"
        indicatorClass="h-4 w-4 items-center justify-center rounded border border-sidebar-border checked:bg-primary"
        indicator={<If when={newBranchCheckout()}><Icon name="check" size={10} color={token("primary-foreground")} /></If>}>
        <Type role="body">Checkout after creating</Type>
      </Checkbox>
      <If when={!!opError()}>
        <Type role="caption" tone="foreground">{opError()}</Type>
      </If>
      <DialogActions>
        <Action onPress={() => branchDialog.set(false)}>Cancel</Action>
        <Action primary testID="history-create-branch" onPress={createBranch}>{busy() ? "Creating…" : "Create branch"}</Action>
      </DialogActions>
    </AppDialog>

    <AppDialog open={deleteDialog()} label="Delete branch" role="alertdialog"
      title={`Delete ${deleteTarget()}?`}
      description="Git refuses a branch that is not merged. The remote branch is not deleted."
      frame="w-[400px]" dismiss={() => deleteDialog.set(false)}>
      <If when={!!opError()}>
        <Type role="caption" tone="foreground">{opError()}</Type>
      </If>
      <DialogActions>
        <Action onPress={() => deleteDialog.set(false)}>Cancel</Action>
        <Action destructive testID="history-delete-branch-confirm" onPress={deleteBranch}>{busy() ? "Deleting…" : "Delete"}</Action>
      </DialogActions>
    </AppDialog>

    <AppDialog open={worktreeDialog()} label="New worktree" title="New worktree" frame="w-[440px]"
      dismiss={() => worktreeDialog.set(false)} closeLabel="Cancel new worktree">
      <Type role="caption">{`Creates a worktree for ${newWorktreeBranch()}.`}</Type>
      <Input testID="history-new-worktree-path" value={newWorktreePath()}
        onChange={({ value }) => newWorktreePath.set(value)} placeholder="Folder"
        accessibilityLabel="Worktree folder" autoFocus
        class="h-(--height-input) rounded-(--radius-control) border border-sidebar-border bg-sidebar px-3 text-(length:--text-body)" />
      <If when={!!opError()}>
        <Type role="caption" tone="foreground">{opError()}</Type>
      </If>
      <DialogActions>
        <Action onPress={() => worktreeDialog.set(false)}>Cancel</Action>
        <Action primary testID="history-create-worktree" onPress={createWorktree}>{busy() ? "Creating…" : "Create worktree"}</Action>
      </DialogActions>
    </AppDialog>

    <AppDialog open={removeDialog()} label="Remove worktree" role="alertdialog"
      title="Remove this worktree?" description="Git refuses when the worktree has uncommitted changes. Its folder is removed, not the branch. Close any group using this folder first."
      frame="w-[420px]" dismiss={() => removeDialog.set(false)}>
      <Type role="body" family="mono" tone="muted-foreground" lines={2}>{removeTarget()}</Type>
      <If when={!!opError()}>
        <Type role="caption" tone="foreground">{opError()}</Type>
      </If>
      <DialogActions>
        <Action onPress={() => removeDialog.set(false)}>Cancel</Action>
        <Action destructive testID="history-remove-worktree" onPress={removeWorktree}>{busy() ? "Removing…" : "Remove"}</Action>
      </DialogActions>
    </AppDialog>
  </View>;
}
