import { Canvas, Clipboard, Scroll, Text, token, View } from "nsh";
import { Button, DiffView, ResizablePanel, ResizablePanelGroup, type TreeEntry } from "nsh/base";
import { Menu } from "nsh/desktop";
import { IconButton } from "../../shared/controls";
import { monoFont } from "../../shared/fonts";
import { Icon } from "../../shared/icons";
import { Type } from "../../shared/type";

import type { GitWorktree } from "../git-types";
import { ROW_HEIGHT, type CommitRow, type FileRow, pathName, fullDate, statusColor, drawGraph, RefChip } from "./common";
import type { HistoryState } from "./state";
import type { HistoryData } from "./data";
import type { HistoryActions } from "./actions";

export function createHistoryViews(state: HistoryState, data: HistoryData, actions: HistoryActions) {
  const {
    props, branches, worktrees, commits, hasMore, logTruncated, selectedOid, detail, selectedFile,
    commitWidth, scope, query, selectedWorktree, expanded, logLoading, loadingMore, detailLoading,
    logError, visibleRows, selectedIndex, selectedPatch, selectedFileInfo, fileRows,
    selectedFileIndex, columnWidths, commitLayoutKey, wide,
  } = state;
  const { loadMore } = data;
  const { openBranchDialog } = actions;
  const renderBranchItem = (entry: TreeEntry) => {
    if (entry.folder) {
      const open = expanded().includes(entry.id);
      const icon = entry.id === "group:tags" ? "gitTag" : "gitBranch";
      return <View class="h-(--height-tab) flex-row items-center gap-1.5"
        style={{ paddingLeft: 4 + entry.depth * 10, paddingRight: 8 }}>
        <Icon name={open ? "down" : "chevron"} size={10} color={token("subtle-foreground")} />
        <Icon name={icon} size={12} color={token("icon")} />
        <Type role="label" class="min-w-0 flex-1" lines={1}>{entry.label}</Type>
      </View>;
    }
    if (entry.id === "scope:all") {
      return <View class="h-(--height-tab) flex-row items-center gap-1.5" style={{ paddingLeft: 4, paddingRight: 8 }}>
        <Icon name="gitCommit" size={12} color={token("icon")} />
        <Type role="body" class="min-w-0 flex-1" lines={1}>All branches</Type>
        <If when={!scope()}><Icon name="check" size={11} color={token("primary")} /></If>
      </View>;
    }
    const branch = branches().find((item) => item.fullName === entry.id.slice("branch:".length));
    if (!branch) return <View />;
    const holder = worktrees().find((worktree) => worktree.branch === branch.name);
    return <View class="h-(--height-tab) flex-row items-center gap-1.5" style={{ paddingLeft: 4, paddingRight: 8 }}>
      <Icon name={branch.kind === "tag" ? "gitTag" : "gitBranch"} size={12} color={token("icon")} />
      <Type role="body" class="min-w-0 flex-1" lines={1}>{branch.name}</Type>
      <If when={branch.current}>
        <Icon name="check" size={11} color={token("primary")} />
      </If>
      <If when={!branch.current && !!holder}>
        <Icon name="gitWorktree" size={11} color={token("subtle-foreground")} />
      </If>
    </View>;
  };

  function WorktreeRowView(props: { worktree: GitWorktree }) {
    const worktree = props.worktree;
    const active = selectedWorktree() === worktree.path;
    return <Button testID={`worktree-${pathName(worktree.path)}`} accessibilityLabel={worktree.path}
      selected={active}
      onPress={() => selectedWorktree.set(worktree.path)}
      class="h-[46px] flex-col justify-center rounded-(--radius-tab) px-2 hover:bg-tab-selected focus-visible:ring-2 ring-sidebar-ring">
      <View class="w-full flex-row items-center gap-1.5">
        <Icon name="gitWorktree" size={12} color={token("icon")} />
        <Type role="body" class="min-w-0 flex-1" lines={1}>{pathName(worktree.path)}</Type>
        <If when={worktree.current}><Icon name="check" size={11} color={token("primary")} /></If>
        <If when={!!worktree.locked}><Icon name="lock" size={10} color={token("subtle-foreground")} /></If>
      </View>
      <View class="w-full flex-row items-center gap-1.5 pl-[18px]">
        <Icon name="gitBranch" size={10} color={token("subtle-foreground")} />
        <Type role="caption" family="mono" tone="subtle-foreground" class="min-w-0 flex-1" lines={1}>
          {worktree.branch ?? (worktree.detached ? "detached" : "—")}
        </Type>
        <If when={worktree.main}><Type role="caption" tone="subtle-foreground">main</Type></If>
      </View>
    </Button>;
  }

  function commitMenu(row: CommitRow, event: { pagePosition: { x: number; y: number } }) {
    void Menu.popup([
      { label: "Copy commit hash", onSelect: () => { void Clipboard.writeText(row.commit.oid); } },
      { label: "Copy subject", onSelect: () => { void Clipboard.writeText(row.commit.subject); } },
      { type: "separator" },
      {
        label: "New branch from this commit…",
        disabled: props.preview,
        onSelect: () => openBranchDialog(row.commit.oid),
      },
    ], event.pagePosition);
  }

  function CommitRowView(props: { row: CommitRow }) {
    const row = props.row;
    const active = row.active;
    const widths = columnWidths();
    return <View class="w-full" onContextMenu={(event) => commitMenu(row, event)}>
      <View testID={`history-commit-row-${row.commit.short}`} class="w-full">
        <Button testID={`history-commit-${row.commit.short}`}
        accessibilityLabel={`${row.commit.short} ${row.commit.subject}`}
        selected={active}
        onPress={() => selectedOid.set(row.commit.oid)}
        class="h-[30px] w-full shrink-0 flex-row items-center border-b border-border hover:bg-tab-selected active:bg-control-pressed focus-visible:ring-1 ring-ring">
        <Canvas style={{ width: widths.graph, height: ROW_HEIGHT }}
          draw={(cx, size) => drawGraph(cx, size, row)} />
        <View class="min-w-0 flex-1 flex-row items-center gap-1.5 px-2">
          <Type role="body" class="min-w-0 flex-1" lines={1}>{row.commit.subject}</Type>
          <Each of={row.commit.refs.slice(0, 2)}>{(ref) => <RefChip label={ref()} />}</Each>
          <If when={row.commit.refs.length > 2}>
            <Type role="caption" tone="subtle-foreground">{`+${row.commit.refs.length - 2}`}</Type>
          </If>
        </View>
        <If when={widths.author > 0}>
          <View style={{ width: widths.author }} class="shrink-0 px-2">
            <Type role="body" lines={1}>{row.author}</Type>
          </View>
        </If>
        <If when={widths.date > 0}>
          <View style={{ width: widths.date }} class="shrink-0 px-2">
            <Type role="caption" tone="subtle-foreground" lines={1}>{row.date}</Type>
          </View>
        </If>
        <If when={widths.hash > 0}>
          <View style={{ width: widths.hash }} class="shrink-0 px-2">
            <Type role="mono" family="mono" tone="subtle-foreground" lines={1}>{row.commit.short}</Type>
          </View>
        </If>
        </Button>
      </View>
    </View>;
  };

  const commitPane = () =>
    <View class="h-full min-h-0 flex-col"
      onLayout={({ bounds }) => {
        const next = Math.round(bounds.size.width);
        if (commitWidth() !== next) commitWidth.set(next);
      }}>
      <If when={!!logError()}>
        <View class="shrink-0 border-b border-border bg-notification-error-muted px-3 py-1">
          <Type role="caption" tone="foreground">{logError()}</Type>
        </View>
      </If>
      <View class="h-[26px] shrink-0 flex-row items-center border-b border-border">
        <View style={{ width: columnWidths().graph }} />
        <View class="min-w-0 flex-1 px-2"><Type role="label">Commit</Type></View>
        <If when={columnWidths().author > 0}>
          <View style={{ width: columnWidths().author }} class="shrink-0 px-2"><Type role="label">Author</Type></View>
        </If>
        <If when={columnWidths().date > 0}>
          <View style={{ width: columnWidths().date }} class="shrink-0 px-2"><Type role="label">Date</Type></View>
        </If>
        <If when={columnWidths().hash > 0}>
          <View style={{ width: columnWidths().hash }} class="shrink-0 px-2"><Type role="label">Hash</Type></View>
        </If>
      </View>
      <If when={visibleRows().length > 0}>
        <Each of={[commitLayoutKey()]} key={(value) => value}>
          {() =>
            <Scroll testID="history-commits" accessibilityLabel="Commit history"
              class="min-h-0 flex-1" contentClass="flex-col">
              <View class="relative flex-col">
                <Each of={visibleRows()} key={(row) => row.commit.oid}>
                  {(row) => <CommitRowView row={row()} />}
                </Each>
                <If when={selectedIndex() >= 0}>
                  <View class="absolute left-0 right-0 bg-primary/20"
                    style={{
                      top: selectedIndex() * ROW_HEIGHT,
                      height: ROW_HEIGHT,
                      pointerEvents: "none",
                      zIndex: 1,
                    }} />
                </If>
                <If when={hasMore()}>
                  <Button testID="history-load-more" onPress={() => { void loadMore(); }}
                    class="h-8 w-full shrink-0 items-center justify-center border-t border-border hover:bg-tab-hover">
                    <Type role="caption" tone="muted-foreground">Load more commits</Type>
                  </Button>
                </If>
                <If when={loadingMore()}>
                  <View class="shrink-0 items-center justify-center py-1">
                    <Type role="caption" tone="subtle-foreground">Loading more commits…</Type>
                  </View>
                </If>
              </View>
            </Scroll>
          }
        </Each>
      </If>
      <If when={visibleRows().length === 0}>
        <View class="flex-1 items-center justify-center gap-2 py-8">
          <Type role="heading">{logLoading() ? "Loading commits…" : commits().length ? "No matching commits." : "No commits yet."}</Type>
          <Type role="body" tone="muted-foreground">
            {commits().length ? hasMore() ? "Search covers loaded commits. Load more to extend it." : "Clear the search to see the history." : "Commit something in a terminal first."}
          </Type>
          <If when={!!query() && hasMore()}>
            <Button testID="history-load-more-empty" disabled={loadingMore()} onPress={() => { void loadMore(); }}
              class="h-(--height-control) rounded-(--radius-control) px-3 hover:bg-control-hover focus-visible:ring-2 ring-ring">
              <Type role="label">{loadingMore() ? "Loading…" : "Load more commits"}</Type>
            </Button>
          </If>
        </View>
      </If>
      <If when={logTruncated()}>
        <View class="shrink-0 border-t border-border px-3 py-1">
          <Type role="caption" tone="subtle-foreground">The log was capped by Git output limits.</Type>
        </View>
      </If>
    </View>;

  const commitInfo = (compact: boolean) => {
    const item = detail();
    if (!item) return <View />;
    return <View class={`min-h-0 flex-col gap-1.5 overflow-hidden px-3 py-2 ${compact ? "shrink-0" : ""}`}>
      <Type role={compact ? "body" : "title"} lines={compact ? 1 : 2}>{item.commit.subject}</Type>
      <View class="flex-row items-center gap-1.5">
        <Type role="mono" family="mono" tone="subtle-foreground">{item.commit.short}</Type>
        <IconButton icon="copy" label="Copy commit hash" onPress={() => { void Clipboard.writeText(item.commit.oid); }} />
        <If when={compact}>
          <Type role="caption" tone="subtle-foreground" class="ml-auto" lines={1}>{fullDate(item.commit.timestamp)}</Type>
        </If>
      </View>
      <If when={!compact}>
        <View class="flex-row items-center gap-1.5">
          <Type role="caption" tone="muted-foreground" lines={1}>{item.commit.author}</Type>
          <Type role="caption" tone="subtle-foreground" lines={1}>{fullDate(item.commit.timestamp)}</Type>
        </View>
      </If>
      <View class="flex-row flex-wrap items-center gap-1">
        <Each of={compact ? item.commit.refs.slice(0, 2) : item.commit.refs}>{(ref) => <RefChip label={ref()} />}</Each>
        <If when={compact && item.commit.refs.length > 2}>
          <Type role="caption" tone="subtle-foreground">{`+${item.commit.refs.length - 2}`}</Type>
        </If>
      </View>
      <If when={!compact && !!item.body}>
        <View class="mt-1">
          <Type role="caption" tone="muted-foreground" lines={4}>{item.body}</Type>
        </View>
      </If>
      <If when={!!item.error}>
        <Type role="caption" tone="foreground">{item.error!}</Type>
      </If>
    </View>;
  };

  function FileRowView(props: { file: FileRow }) {
    const file = props.file;
    const active = file.active;
    return <View class="w-full">
      <Button
        testID={`history-file-${file.path.replace(/[^a-zA-Z0-9]+/g, "-")}`}
        accessibilityLabel={file.path}
        selected={active}
        onPress={() => selectedFile.set(file.path)}
        class="h-6 w-full flex-row items-center gap-1.5 px-3 hover:bg-tab-selected active:bg-control-pressed focus-visible:ring-1 ring-ring">
        <Text style={{
          width: 12, textAlign: "center", fontFamily: monoFont(),
          fontSize: token("text-caption"), color: statusColor(file.status),
        }}>{file.status}</Text>
        <Type role="caption" family="mono" tone={active ? "foreground" : "muted-foreground"} class="min-w-0 flex-1" lines={1}>
          {file.oldPath ? `${file.oldPath} → ${file.path}` : file.path}
        </Type>
      </Button>
    </View>;
  }

  const changedFiles = () => {
    const item = detail();
    if (!item) return <View />;
    return <View class="h-full min-h-0 flex-col border-t border-border">
      <View class="h-(--height-status) shrink-0 flex-row items-center px-3">
        <Type role="label" class="min-w-0 flex-1">Changed files</Type>
        <Type role="mono">{`${item.files.length}`}</Type>
      </View>
      <If when={item.files.length > 0}>
        <Scroll class="min-h-0 flex-1" contentClass="flex-col py-1">
          <View class="relative flex-col">
            <Each of={fileRows()} key={(file) => file.path}>
              {(file) => <FileRowView file={file()} />}
            </Each>
            <If when={selectedFileIndex() >= 0}>
              <View class="absolute left-0 right-0 bg-primary/20"
                style={{
                  top: selectedFileIndex() * 24,
                  height: 24,
                  pointerEvents: "none",
                  zIndex: 1,
                }} />
            </If>
          </View>
        </Scroll>
      </If>
      <If when={item.files.length === 0}>
        <View class="flex-1 items-center justify-center">
          <Type role="caption" tone="subtle-foreground">No changed paths reported.</Type>
        </View>
      </If>
    </View>;
  };

  const patchPane = () => {
    const item = detail();
    if (!item) return <View />;
    return <View class="h-full min-h-0 flex-col border-t border-border">
      <View class="h-(--height-status) shrink-0 flex-row items-center gap-1.5 border-b border-border px-3">
        <Icon name="file" size={12} color={token("icon")} />
        <Type role="caption" family="mono" tone="muted-foreground" class="min-w-0 flex-1" lines={1}>
          {selectedFile() ?? "Commit patch"}
        </Type>
        <If when={!!selectedFileInfo()}>
          <Text style={{
            fontFamily: monoFont(), fontSize: token("text-caption"),
            color: statusColor(selectedFileInfo()!.status),
          }}>{selectedFileInfo()!.status}</Text>
        </If>
      </View>
      <If when={!!selectedPatch()}>
        <DiffView patch={selectedPatch()} layout="unified" softWrap contextLines={3}
          fontFamily={monoFont()} fontSize={token("text-body")} lineHeight={token("leading-body") + 4}
          class="min-h-0 flex-1" />
      </If>
      <If when={!selectedPatch()}>
        <View class="flex-1 items-center justify-center gap-2 px-4">
          <Type role="body" tone="muted-foreground" class="text-center">
            {selectedFile()
              ? "No text diff for this file. Open it in a terminal to inspect it."
              : item.commit.parents.length > 1
                ? "Merge commit with no text diff against its first parent."
                : "No text diff. Open the path in a terminal to inspect it."}
          </Type>
        </View>
      </If>
      <If when={item.truncated}>
        <View class="shrink-0 border-t border-border px-3 py-1">
          <Type role="caption" tone="subtle-foreground">Patch capped at 2 MB.</Type>
        </View>
      </If>
    </View>;
  };

  const detailEmpty = () =>
    <View class="flex-1 items-center justify-center gap-2 px-4">
      <Icon name="gitCommit" size={18} />
      <Type role="heading">Select a commit.</Type>
      <Type role="body" tone="muted-foreground" class="text-center">Its message, files and patch appear here.</Type>
    </View>;

  const detailLoadingState = () =>
    <View class="flex-1 items-center justify-center">
      <Type role="body" tone="muted-foreground">Loading commit…</Type>
    </View>;

  const detailWide = () =>
    <View testID="history-detail-wide" class="h-full min-h-0 flex-col border-l border-border">
      <If when={!detail() && !detailLoading()}>{detailEmpty()}</If>
      <If when={detailLoading() && !detail()}>{detailLoadingState()}</If>
      <If when={!!detail()}>
        <ResizablePanelGroup axis="vertical" class="h-full min-h-0"
          handleClass="bg-transparent group-hover:bg-sidebar-border group-active:bg-ring"
          handleAccessibilityLabel="Resize commit details and patch">
          <ResizablePanel size={182} minSize={110}>{commitInfo(false)}</ResizablePanel>
          <ResizablePanel size={160} minSize={80}>{changedFiles()}</ResizablePanel>
          <ResizablePanel minSize={140}>{patchPane()}</ResizablePanel>
        </ResizablePanelGroup>
      </If>
    </View>;

  const detailNarrow = () =>
    <View testID="history-detail-narrow" class="h-full min-h-0 flex-col border-t border-border">
      <If when={!detail() && !detailLoading()}>{detailEmpty()}</If>
      <If when={detailLoading() && !detail()}>{detailLoadingState()}</If>
      <If when={!!detail()}>
        <ResizablePanelGroup axis="horizontal" class="h-full min-h-0"
          handleClass="bg-transparent group-hover:bg-sidebar-border group-active:bg-ring"
          handleAccessibilityLabel="Resize commit details and patch">
          <ResizablePanel size={320} minSize={220} maxSize={460}>
            <View class="h-full min-h-0 flex-col border-r border-border">
              {commitInfo(true)}
              {changedFiles()}
            </View>
          </ResizablePanel>
          <ResizablePanel minSize={240}>{patchPane()}</ResizablePanel>
        </ResizablePanelGroup>
      </If>
    </View>;

  return { renderBranchItem, WorktreeRowView, commitPane, detailWide, detailNarrow };
}
