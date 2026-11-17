import { computed, signal } from "nsh";
import type { TreeItem } from "nsh/base";

import { layoutCommits, type GraphRow } from "../commit-graph";
import type { GitBranch, GitCommit, GitCommitDetail, GitRepoStatus, GitWorktree } from "../git-types";
import { LANE_GAP, GRAPH_LEFT, type CommitRow, type FileRow, type HistoryProps, formatDate, patchForFile } from "./common";

export function createHistoryState(props: HistoryProps) {
  const store = props.workspace;
  const sessionVersion = props.session?.version ?? 0;
  const cached = props.session?.history?.version === sessionVersion ? props.session.history : undefined;
  const saved = props.session?.historyPreferences;

  const groupId = () => store.workspaceKey();
  const group = () => store.group(groupId());
  const cwd = () => props.root?.() ?? group()?.cwd ?? "~";

  const status = signal<GitRepoStatus | null>(cached?.status ?? null);
  const branches = signal<GitBranch[]>(cached?.branches ?? []);
  const worktrees = signal<GitWorktree[]>(cached?.worktrees ?? []);
  const commits = signal<GitCommit[]>(cached?.commits ?? []);
  const graphRows = signal<GraphRow[] | null>(cached?.graphRows ?? null);
  const graphMaxLanes = signal(cached?.graphMaxLanes ?? 0);
  const graphClipped = signal(cached?.graphClipped ?? false);
  const hasMore = signal(cached?.hasMore ?? false);
  const logTruncated = signal(cached?.logTruncated ?? false);
  const selectedOid = signal<string | null>(cached?.selectedOid ?? saved?.selectedOid ?? null);
  const detail = signal<GitCommitDetail | null>(cached?.detail ?? null);
  const selectedFile = signal<string | null>(cached?.selectedFile ?? saved?.selectedFile ?? null);
  const commitWidth = signal(0);
  const scrollOffset = signal(cached?.scrollOffset ?? saved?.scrollOffset ?? 0);
  const scope = signal(cached?.scope ?? saved?.scope ?? "");
  const query = signal(cached?.query ?? saved?.query ?? "");
  const refMatches = signal<Set<string>>(new Set());
  const searchOpen = signal(cached?.searchOpen ?? saved?.searchOpen ?? false);
  const railTab = signal<"branches" | "worktrees">(cached?.railTab ?? saved?.railTab ?? "branches");
  const selectedBranch = signal<string | null>(cached?.selectedBranch ?? saved?.selectedBranch ?? null);
  const selectedWorktree = signal<string | null>(cached?.selectedWorktree ?? saved?.selectedWorktree ?? null);
  const expanded = signal<string[]>(cached?.expanded ?? saved?.expanded ?? ["group:local", "group:remote", "group:tags"]);
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
      refMatches().has(commit.oid)
      || commit.subject.toLowerCase().includes(text)
      || commit.author.toLowerCase().includes(text)
      || commit.short.toLowerCase().includes(text)
      || commit.refs.some((ref) => ref.toLowerCase().includes(text))
    );
  });
  const loadedGraph = computed(() => {
    const rows = graphRows();
    return rows ? { rows, maxLanes: graphMaxLanes(), clipped: graphClipped() } : layoutCommits(commits());
  });
  const visibleGraph = computed(() => {
    const graph = loadedGraph();
    if (!query().trim()) return graph;
    const matches = new Set(filtered().map(commit => commit.oid));
    return { ...graph, rows: graph.rows.filter(row => matches.has(row.commit.oid)) };
  });
  const visibleRows = computed<CommitRow[]>(() => {
    return visibleGraph().rows.map((row) => ({
      ...row,
      author: row.commit.author,
      date: formatDate(row.commit.timestamp),
      short: row.commit.short,
      subject: row.commit.subject,
    }));
  });
  const commitsByOid = computed(() => new Map(commits().map(commit => [commit.oid, commit])));
  const selectedCommit = computed(() => commitsByOid().get(selectedOid() ?? "") ?? detail()?.commit);
  const selectedRefs = computed(() => selectedCommit()?.refs ?? []);
  const selectedRefCount = computed(() => selectedCommit()?.refCount ?? selectedRefs().length);
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
  const visibleGraphWidth = computed(() => query().trim() ? 46 : Math.max(46, GRAPH_LEFT + Math.max(1, visibleGraph().maxLanes) * LANE_GAP + 10));

  const branchesByRef = computed(() => new Map(branches().map(branch => [branch.fullName, branch])));

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
      graph: Math.min(visibleGraphWidth(), Math.max(46, width * 0.25)),
      author: width >= 500 ? 110 : 0,
      date: width >= 680 ? 100 : 0,
      hash: width >= 800 ? 74 : 0,
    };
  });

  const wide = computed(() => (store.canvasSize().width || 1200) >= 1000);

  return {
    props, sessionVersion, cached, scrollOffset, store, groupId, group, cwd, status, branches, worktrees, commits, graphRows, graphMaxLanes, graphClipped, hasMore, logTruncated,
    selectedOid, detail, selectedFile, commitWidth, scope, query, refMatches, searchOpen, railTab,
    selectedBranch, selectedWorktree, expanded, loading, logLoading, loadingMore, detailLoading,
    error, logError, opError, networkAction, busy, branchDialog, deleteDialog, worktreeDialog,
    removeDialog, newBranchName, newBranchCheckout, branchStart, newWorktreePath,
    newWorktreeBranch, removeTarget, deleteTarget, epoch, failure, repo, filtered, visibleGraph,
    visibleRows, selectedRefs, selectedRefCount, selectedPatch, selectedFileInfo, fileRows, selectedFileIndex,
    selectedWorktreeIndex, selectedBranchInfo, selectedBranchWorktree, canCheckout,
    canDeleteBranch, selectedWorktreeInfo, canRemoveWorktree, scopeLabel, startLabel,
    visibleGraphWidth, branchItems, branchesByRef, columnWidths, wide,
  };
}

export type HistoryState = ReturnType<typeof createHistoryState>;
