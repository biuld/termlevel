import { computed, signal } from "nsh";
import type { TreeItem } from "nsh/base";

import { layoutCommits } from "../commit-graph";
import type { GitBranch, GitCommit, GitCommitDetail, GitRepoStatus, GitWorktree } from "../git-types";
import { LANE_GAP, GRAPH_LEFT, type CommitRow, type FileRow, type HistoryProps, formatDate, patchForFile } from "./common";

export function createHistoryState(props: HistoryProps) {
  const store = props.workspace;
  const groupId = () => store.workspaceKey();
  const group = () => store.group(groupId());
  const cwd = () => group()?.cwd ?? "~";

  const status = signal<GitRepoStatus | null>(null);
  const branches = signal<GitBranch[]>([]);
  const worktrees = signal<GitWorktree[]>([]);
  const commits = signal<GitCommit[]>([]);
  const hasMore = signal(false);
  const logTruncated = signal(false);
  const selectedOid = signal<string | null>(null);
  const detail = signal<GitCommitDetail | null>(null);
  const selectedFile = signal<string | null>(null);
  const commitWidth = signal(0);
  const scope = signal("");
  const query = signal("");
  const searchOpen = signal(false);
  const railTab = signal<"branches" | "worktrees">("branches");
  const selectedBranch = signal<string | null>(null);
  const selectedWorktree = signal<string | null>(null);
  const expanded = signal<string[]>(["group:local", "group:remote", "group:tags"]);
  const loading = signal(true);
  const logLoading = signal(false);
  const loadingMore = signal(false);
  const detailLoading = signal(false);
  const error = signal("");
  const logError = signal("");
  const opError = signal("");
  const networkAction = signal<"fetch" | "pull" | "push" | null>(null);
  const busy = signal(false);
  const branchDialog = signal(false);
  const deleteDialog = signal(false);
  const worktreeDialog = signal(false);
  const removeDialog = signal(false);
  const newBranchName = signal("");
  const newBranchCheckout = signal(true);
  const branchStart = signal("");
  const newWorktreePath = signal("");
  const newWorktreeBranch = signal("");
  const removeTarget = signal("");
  const deleteTarget = signal("");
  const epoch = signal(0);

  const failure = () => error() || status()?.error || "";
  const repo = () => !!status()?.repo;
  const filtered = computed(() => {
    const text = query().trim().toLowerCase();
    if (!text) return commits();
    return commits().filter((commit) =>
      commit.subject.toLowerCase().includes(text)
      || commit.author.toLowerCase().includes(text)
      || commit.short.toLowerCase().includes(text)
      || commit.refs.some((ref) => ref.toLowerCase().includes(text))
    );
  });
  const visibleGraph = computed(() => layoutCommits(filtered()));
  const visibleRows = computed<CommitRow[]>(() => {
    const activeOid = selectedOid();
    return visibleGraph().rows.map((row) => ({
      ...row,
      author: row.commit.author,
      date: formatDate(row.commit.timestamp),
      short: row.commit.short,
      subject: row.commit.subject,
      active: row.commit.oid === activeOid,
    }));
  });
  const selectedIndex = computed(() => visibleRows().findIndex((row) => row.active));
  const selectedPatch = computed(() => {
    const item = detail();
    const path = selectedFile();
    if (!item) return "";
    if (!path) return item.patch;
    return patchForFile(item.patch, path);
  });
  const selectedFileInfo = computed(() => detail()?.files.find((file) => file.path === selectedFile()));
  const fileRows = computed<FileRow[]>(() => {
    const selected = selectedFile();
    return (detail()?.files ?? []).map((file) => ({ ...file, active: file.path === selected }));
  });
  const selectedFileIndex = computed(() => fileRows().findIndex((file) => file.active));
  const selectedWorktreeIndex = computed(() => worktrees().findIndex((worktree) => worktree.path === selectedWorktree()));
  const currentCommits = computed(() => {
    const reachable = new Set<string>();
    const byOid = new Map(commits().map((commit) => [commit.oid, commit]));
    const head = branches().find((branch) => branch.current)?.target;
    if (!head) return reachable;
    const pending = [head];
    while (pending.length) {
      const oid = pending.pop()!;
      if (reachable.has(oid)) continue;
      reachable.add(oid);
      const commit = byOid.get(oid);
      if (commit) pending.push(...commit.parents);
    }
    return reachable;
  });
  const selectedBranchInfo = computed(() => branches().find((branch) => branch.fullName === selectedBranch()));
  const selectedBranchWorktree = computed(() => {
    const branch = selectedBranchInfo();
    return branch ? worktrees().find((worktree) => worktree.branch === branch.name) : undefined;
  });
  const canCheckout = computed(() => {
    const branch = selectedBranchInfo();
    if (!branch || branch.current || branch.kind === "tag") return false;
    const holder = selectedBranchWorktree();
    return !holder || holder.current;
  });
  const canDeleteBranch = computed(() => {
    const branch = selectedBranchInfo();
    if (!branch || branch.kind !== "local" || branch.current) return false;
    return !selectedBranchWorktree();
  });
  const selectedWorktreeInfo = computed(() => worktrees().find((worktree) => worktree.path === selectedWorktree()));
  const canRemoveWorktree = computed(() => {
    const worktree = selectedWorktreeInfo();
    return !!worktree && !worktree.main && !worktree.current && !worktree.locked;
  });
  const scopeLabel = computed(() => {
    const full = scope();
    if (!full) return "All branches";
    return branches().find((branch) => branch.fullName === full)?.name ?? full;
  });
  const startLabel = computed(() => {
    const start = branchStart();
    if (start) return /^[0-9a-f]{7,64}$/i.test(start) ? start.slice(0, 7) : start;
    const full = scope();
    if (full) return branches().find((branch) => branch.fullName === full)?.name ?? full;
    return status()?.branch ?? "HEAD";
  });
  const visibleGraphWidth = computed(() => Math.max(46, GRAPH_LEFT + Math.max(1, visibleGraph().maxLanes) * LANE_GAP + 10));

  const branchItems = computed<TreeItem[]>(() => {
    const items: TreeItem[] = [{ id: "scope:all", label: "All branches" }];
    const groups: Array<{ id: string; label: string; kind: GitBranch["kind"] }> = [
      { id: "group:local", label: "Local", kind: "local" },
      { id: "group:remote", label: "Remote", kind: "remote" },
      { id: "group:tags", label: "Tags", kind: "tag" },
    ];
    for (const section of groups) {
      const members = branches().filter((branch) => branch.kind === section.kind);
      if (!members.length) continue;
      items.push({
        id: section.id,
        label: `${section.label} · ${members.length}`,
        children: members.map((branch) => ({ id: `branch:${branch.fullName}`, label: branch.name })),
      });
    }
    return items;
  });

  const columnWidths = computed(() => {
    const width = commitWidth() || store.canvasSize().width || 1200;
    return {
      graph: visibleGraphWidth(),
      author: width >= 500 ? 110 : 0,
      date: width >= 680 ? 100 : 0,
      hash: width >= 800 ? 74 : 0,
    };
  });

  const commitLayoutKey = computed(() => {
    const widths = columnWidths();
    return `${widths.graph}:${widths.author}:${widths.date}:${widths.hash}`;
  });

  const wide = computed(() => (store.canvasSize().width || 1200) >= 1000);

  return {
    props, store, groupId, group, cwd, status, branches, worktrees, commits, hasMore, logTruncated,
    selectedOid, detail, selectedFile, commitWidth, scope, query, searchOpen, railTab,
    selectedBranch, selectedWorktree, expanded, loading, logLoading, loadingMore, detailLoading,
    error, logError, opError, networkAction, busy, branchDialog, deleteDialog, worktreeDialog,
    removeDialog, newBranchName, newBranchCheckout, branchStart, newWorktreePath,
    newWorktreeBranch, removeTarget, deleteTarget, epoch, failure, repo, filtered, visibleGraph,
    visibleRows, selectedIndex, selectedPatch, selectedFileInfo, fileRows, selectedFileIndex,
    selectedWorktreeIndex, currentCommits, selectedBranchInfo, selectedBranchWorktree, canCheckout,
    canDeleteBranch, selectedWorktreeInfo, canRemoveWorktree, scopeLabel, startLabel,
    visibleGraphWidth, branchItems, columnWidths, commitLayoutKey, wide,
  };
}

export type HistoryState = ReturnType<typeof createHistoryState>;
