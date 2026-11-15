import { batch, Canvas, Clipboard, cleanup, computed, effect, lifecycle, List, signal, Text, token, untracked, View, type CanvasRenderingContext2D as CanvasContext } from "nsh";
import {
  Button,
  Checkbox,
  DiffView,
  Input,
  ResizablePanel,
  ResizablePanelGroup,
  Tree,
  type TreeEntry,
  type TreeItem,
} from "nsh/base";
import { Menu } from "nsh/desktop";
import { Action, AppDialog, DialogActions, Hint, IconButton } from "../shared/controls";
import { monoFont } from "../shared/fonts";
import { Icon } from "../shared/icons";
import { Type } from "../shared/type";
import type { Workspace } from "../workspace/workspace";
import { layoutCommits, type GraphRow } from "./commit-graph";
import type {
  GitBranch,
  GitCommit,
  GitCommitDetail,
  GitLog,
  GitRepoStatus,
  GitTransport,
  GitWorktree,
} from "./git-types";

const LOG_PAGE = 200;
const ROW_HEIGHT = 30;
const LANE_GAP = 14;
const GRAPH_LEFT = 12;

interface CommitRow extends GraphRow {
  author: string;
  date: string;
  short: string;
  subject: string;
}

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message;
  return typeof error === "string" ? error : "Git request failed.";
}

function pathName(path: string) {
  const trimmed = path.trim().replace(/[\\/]+$/, "");
  const cut = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  return (cut >= 0 ? trimmed.slice(cut + 1) : trimmed) || trimmed;
}

function formatDate(timestamp: number) {
  const date = new Date(timestamp * 1000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function fullDate(timestamp: number) {
  const date = new Date(timestamp * 1000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function shortRef(ref: string) {
  if (ref.startsWith("HEAD -> ")) return ref.slice(8);
  if (ref.startsWith("tag: ")) return ref.slice(5);
  return ref;
}

function refColor(ref: string) {
  if (ref.startsWith("HEAD")) return token("primary");
  if (ref.startsWith("tag: ")) return token("notification-warning");
  if (ref.includes("/")) return token("group-purple");
  return token("group-green");
}

function statusColor(status: string) {
  switch (status) {
    case "A": return token("group-green");
    case "M": case "T": return token("notification-warning");
    case "D": case "U": return token("destructive");
    case "R": case "C": return token("primary");
    default: return token("muted-foreground");
  }
}

/** The unified-diff section for one path, or empty when the commit has no text diff for it. */
function patchForFile(patch: string, path: string): string {
  if (!patch) return "";
  const sections: string[][] = [];
  let section: string[] | null = null;
  for (const line of patch.split("\n")) {
    if (line.startsWith("diff --git ")) {
      if (section) sections.push(section);
      section = [line];
    } else if (section) {
      section.push(line);
    }
  }
  if (section) sections.push(section);
  // A hand-written patch (or a preview fixture) may have no `diff --git` lines.
  if (!sections.length) sections.push(patch.split("\n"));
  for (const lines of sections) {
    if (lines.some((line) => line.startsWith("Binary files ") || line.startsWith("GIT binary patch"))) {
      continue;
    }
    const paths = new Set<string>();
    for (const line of lines) {
      if (line.startsWith("diff --git ")) {
        const match = /^diff --git a\/(.+?) b\/(.+)$/.exec(line);
        if (match) {
          paths.add(match[1]);
          paths.add(match[2]);
        }
      } else if (line.startsWith("+++ b/") || line.startsWith("--- a/")) {
        paths.add(line.slice(6).replace(/\t.*$/, ""));
      } else if (line.startsWith("rename to ") || line.startsWith("copy to ")) {
        paths.add(line.slice(line.indexOf("to ") + 3));
      } else if (line.startsWith("rename from ") || line.startsWith("copy from ")) {
        paths.add(line.slice(line.indexOf("from ") + 5));
      }
    }
    if (paths.has(path)) return lines.join("\n");
  }
  return "";
}

function laneColor(lane: number) {
  const colors = [
    token("group-green"),
    token("group-blue"),
    token("group-purple"),
    token("group-orange"),
    token("group-pink"),
    token("group-cyan"),
    token("group-red"),
    token("group-yellow"),
  ];
  return colors[lane % colors.length];
}

function drawGraph(cx: CanvasContext, size: { width: number; height: number }, row: GraphRow) {
  const mid = size.height / 2;
  const x = (lane: number) => GRAPH_LEFT + lane * LANE_GAP;
  const line = (lane: number, from: number, to: number, color: string) => {
    cx.beginPath();
    cx.moveTo(x(lane), from);
    cx.lineTo(x(lane), to);
    cx.strokeStyle = color;
    cx.stroke();
  };
  cx.lineWidth = 1.5;
  cx.lineCap = "round";
  cx.lineJoin = "round";
  for (const lane of row.through) line(lane, 0, size.height, laneColor(lane));
  for (let lane = 0; lane < row.before.length; lane++) {
    if (lane === row.lane) {
      line(lane, 0, mid, laneColor(lane));
    } else if (!row.after.includes(row.before[lane])) {
      line(lane, 0, mid, laneColor(lane));
    }
  }
  for (const edge of row.edges) {
    if (edge.to === row.lane) {
      line(edge.to, mid, size.height, laneColor(edge.to));
      continue;
    }
    cx.beginPath();
    cx.moveTo(x(row.lane), mid);
    cx.bezierCurveTo(
      x(row.lane), mid + (size.height - mid) * 0.35,
      x(edge.to), size.height - (size.height - mid) * 0.35,
      x(edge.to), size.height,
    );
    cx.strokeStyle = laneColor(edge.to);
    cx.stroke();
  }
  cx.beginPath();
  cx.arc(x(row.lane), mid, 3.5, 0, Math.PI * 2);
  cx.fillStyle = laneColor(row.lane);
  cx.fill();
}

function RefChip(props: { label: string }) {
  const color = refColor(props.label);
  return <View class="shrink-0 rounded-(--radius-tab) border px-1" style={{ borderColor: color }}>
    <Text style={{
      fontFamily: monoFont(),
      fontSize: token("text-caption"),
      lineHeight: token("leading-caption"),
      color,
      numberOfLines: 1,
    }}>{shortRef(props.label)}</Text>
  </View>;
}

/** Branches, worktrees, commit lanes and the selected commit's patch for one group. */
export function HistoryPanel(props: { workspace: Workspace; git: GitTransport; preview: boolean }) {
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
  const hoveredCommit = signal<string | null>(null);
  const hoveredFile = signal<string | null>(null);
  const commitWidth = signal(0);
  const scope = signal("");
  const query = signal("");
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

  let loadGeneration = 0;
  let logGeneration = 0;
  let detailGeneration = 0;
  let lastPath = "";

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
  const visibleRows = computed<CommitRow[]>(() => visibleGraph().rows.map((row) => ({
    ...row,
    author: row.commit.author,
    date: formatDate(row.commit.timestamp),
    short: row.commit.short,
    subject: row.commit.subject,
  })));
  const selectedPatch = computed(() => {
    const item = detail();
    const path = selectedFile();
    if (!item) return "";
    if (!path) return item.patch;
    return patchForFile(item.patch, path);
  });
  const selectedFileInfo = computed(() => detail()?.files.find((file) => file.path === selectedFile()));
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

  const renderWorktreeRow = (worktree: GitWorktree) =>
    <Button testID={`worktree-${pathName(worktree.path)}`} accessibilityLabel={worktree.path}
      selected={selectedWorktree() === worktree.path}
      onPress={() => selectedWorktree.set(worktree.path)}
      class="h-[46px] flex-col justify-center rounded-(--radius-tab) px-2 hover:bg-tab-hover focus-visible:ring-2 ring-sidebar-ring"
      classList={{ "bg-tab-selected": selectedWorktree() === worktree.path }}>
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

  const columnWidths = computed(() => {
    const width = commitWidth() || store.canvasSize().width || 1200;
    return {
      graph: visibleGraphWidth(),
      author: width >= 500 ? 110 : 0,
      date: width >= 680 ? 100 : 0,
      hash: width >= 800 ? 74 : 0,
    };
  });

  const wide = computed(() => (store.canvasSize().width || 1200) >= 1000);

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

  const renderCommitRow = (row: CommitRow) => {
    const active = selectedOid() === row.commit.oid;
    const widths = columnWidths();
    return <View class="w-full" onContextMenu={(event) => commitMenu(row, event)}>
      <View testID={`history-commit-row-${row.commit.short}`} class="w-full"
        onMouseEnter={() => hoveredCommit.set(row.commit.oid)}
        onMouseLeave={() => { if (hoveredCommit() === row.commit.oid) hoveredCommit.set(null); }}>
        <Button testID={`history-commit-${row.commit.short}`}
        accessibilityLabel={`${row.commit.short} ${row.commit.subject}`}
        selected={active}
        onPress={() => selectedOid.set(row.commit.oid)}
        class="h-[30px] w-full shrink-0 flex-row items-center border-b border-border active:bg-control-pressed focus-visible:ring-1 ring-ring"
        classList={{
          "bg-primary/15": active,
          "bg-tab-selected": !active && hoveredCommit() === row.commit.oid,
          "bg-tab-hover": !active && hoveredCommit() !== row.commit.oid && currentCommits().has(row.commit.oid),
        }}>
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
        <List testID="history-commits" accessibilityLabel="Commit history"
          each={visibleRows()} keyExtractor={(row) => row.commit.oid}
          estimatedItemSize={ROW_HEIGHT} class="min-h-0 flex-1"
          onLoadMore={() => { void loadMore(); }}>
          {(item) => renderCommitRow(item())}
        </List>
      </If>
      <If when={visibleRows().length === 0}>
        <View class="flex-1 items-center justify-center gap-2 py-8">
          <Type role="heading">{logLoading() ? "Loading commits…" : commits().length ? "No matching commits." : "No commits yet."}</Type>
          <Type role="body" tone="muted-foreground">
            {commits().length ? "Clear the search to see the history." : "Commit something in a terminal first."}
          </Type>
        </View>
      </If>
      <If when={loadingMore()}>
        <View class="shrink-0 items-center justify-center py-1">
          <Type role="caption" tone="subtle-foreground">Loading more commits…</Type>
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

  const changedFiles = () => {
    const item = detail();
    if (!item) return <View />;
    return <View class="h-full min-h-0 flex-col border-t border-border">
      <View class="h-(--height-status) shrink-0 flex-row items-center px-3">
        <Type role="label" class="min-w-0 flex-1">Changed files</Type>
        <Type role="mono">{`${item.files.length}`}</Type>
      </View>
      <If when={item.files.length > 0}>
        <List each={item.files} keyExtractor={(file) => file.path}
          class="min-h-0 flex-1" contentClass="py-1">
          {(entry) => {
            const file = entry();
            const active = selectedFile() === file.path;
            return <View class="w-full"
              onMouseEnter={() => hoveredFile.set(file.path)}
              onMouseLeave={() => { if (hoveredFile() === file.path) hoveredFile.set(null); }}>
              <Button
                testID={`history-file-${file.path.replace(/[^a-zA-Z0-9]+/g, "-")}`}
                accessibilityLabel={file.path}
                selected={active}
                onPress={() => selectedFile.set(file.path)}
                class="h-6 w-full flex-row items-center gap-1.5 px-3 active:bg-control-pressed focus-visible:ring-1 ring-ring"
                classList={{
                  "bg-primary/15": active,
                  "bg-tab-selected": !active && hoveredFile() === file.path,
                }}>
                <Text style={{
                  width: 12, textAlign: "center", fontFamily: monoFont(),
                  fontSize: token("text-caption"), color: statusColor(file.status),
                }}>{file.status}</Text>
                <Type role="caption" family="mono" tone={active ? "foreground" : "muted-foreground"} class="min-w-0 flex-1" lines={1}>
                  {file.oldPath ? `${file.oldPath} → ${file.path}` : file.path}
                </Type>
              </Button>
            </View>;
          }}
        </List>
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
        <DiffView patch={selectedPatch().trimEnd()} layout="unified" softWrap contextLines={3}
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

  return <View testID="history-panel" class="min-h-0 flex-1 flex-col">
    <View class="h-(--height-toolbar) shrink-0 flex-row items-center gap-2 border-b border-border px-3">
      <Icon name="gitBranch" size={14} color={token("icon")} />
      <If when={repo()}>
        <Type role="body" family="mono" tone="foreground">{scopeLabel()}</Type>
        <If when={!!scope()}>
          <IconButton icon="close" label="Show all branches" testID="history-clear-scope"
            onPress={() => selectScope("scope:all")} />
        </If>
        <View class="h-3 w-px bg-border" />
        <If when={status()?.detached}>
          <Type role="caption" tone="subtle-foreground">{`HEAD@${status()?.branch ?? ""}`}</Type>
        </If>
        <If when={!status()?.detached && !scope()}>
          <Type role="caption" tone="subtle-foreground">{status()?.branch ?? ""}</Type>
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
      </If>
      <View class="ml-auto flex-row items-center gap-1.5">
        <Icon name="search" size={12} color={token("icon")} />
        <Input testID="history-search" value={query()} onChange={({ value }) => query.set(value)}
          placeholder="Search commits" accessibilityLabel="Search commits"
          class="h-(--height-control) w-[180px] rounded-(--radius-control) border border-sidebar-border bg-sidebar px-2" />
        <If when={logLoading() || loading()}>
          <Type role="caption" tone="subtle-foreground">Loading…</Type>
        </If>
        <IconButton icon="refresh" label="Refresh history" testID="history-refresh" onPress={refresh} />
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
              <View class="shrink-0 border-t border-border px-2 py-2">
                <View class="flex-row items-center gap-1">
                  <Button testID="history-checkout" disabled={props.preview || busy() || !canCheckout()}
                    onPress={checkoutSelected}
                    class="h-(--height-action) flex-1 items-center justify-center rounded-(--radius-control) px-2 focus-visible:ring-2 ring-ring"
                    classList={{ "bg-tab-selected": canCheckout() && !props.preview, "opacity-40": !canCheckout() || props.preview }}>
                    <Type role="label" tone={canCheckout() ? "foreground" : "subtle-foreground"}>Checkout</Type>
                  </Button>
                  <IconButton icon="gitWorktree" label="New worktree from this branch" testID="history-branch-worktree"
                    disabled={props.preview || !selectedBranchInfo() || selectedBranchInfo()?.kind !== "local"}
                    onPress={openWorktreeDialog} />
                  <IconButton icon="trash" label="Delete branch" testID="history-delete-branch"
                    disabled={props.preview || busy() || !canDeleteBranch()} onPress={openDeleteDialog} />
                </View>
                <If when={!!selectedBranchInfo()}>
                  <Type role="caption" tone="subtle-foreground" class="mt-1 px-1" lines={1}>
                    {selectedBranchInfo()!.kind === "local"
                      ? (selectedBranchInfo()!.upstream ? `upstream ${selectedBranchInfo()!.upstream}` : "local branch")
                      : selectedBranchInfo()!.kind === "remote" ? "remote branch" : "tag"}
                  </Type>
                </If>
                <If when={!!selectedBranchWorktree() && !selectedBranchInfo()?.current}>
                  <Type role="caption" tone="subtle-foreground" class="mt-0.5 px-1" lines={1}>
                    {`Checked out in ${pathName(selectedBranchWorktree()!.path)}.`}
                  </Type>
                </If>
                <If when={(status()?.changes.length ?? 0) > 0}>
                  <Type role="caption" tone="subtle-foreground" class="mt-0.5 px-1" lines={1}>
                    Working tree has changes.
                  </Type>
                </If>
              </View>
            </If>

            <If when={railTab() === "worktrees"}>
              <If when={worktrees().length > 0}>
                <List each={worktrees()} keyExtractor={(worktree: GitWorktree) => worktree.path}
                  testID="history-worktree-list" class="min-h-0 flex-1" contentClass="py-1">
                  {(item) => renderWorktreeRow(item())}
                </List>
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
              <ResizablePanel size={300} minSize={170} maxSize={480}>{detailNarrow()}</ResizablePanel>
            </ResizablePanelGroup>
          </If>
        </ResizablePanel>

        <ResizablePanel size={390} minSize={300} maxSize={540} visible={wide()}>
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
