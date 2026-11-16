import { cleanup, effect, signal, untracked, View } from "nsh";
import { Button, Popover } from "nsh/base";
import { Type } from "../shared/type";
import type { Workspace } from "../workspace/state";
import { GitPanel } from "./GitPanel";
import { HistoryPanel } from "./HistoryPanel";
import type { GitTransport } from "./git-types";
import { scheduleGitTasks } from "./scheduler";
import { gitOperation, gitOperationBusy } from "./operation-gate";

/** One repository selection shared by Changes and History for the active group. */
export function GitWorkspace(props: {
  workspace: Workspace; git: GitTransport; preview: boolean;
  notify: (level: "info" | "error", title: string, body: string) => void;
}) {
  const paths = signal<string[]>([]);
  const selected = signal("");
  const loading = signal(true);
  const scanning = signal(false);
  const error = signal("");
  const open = signal(false);
  const refreshTick = signal(0);
  const fetchError = signal("");
  let disposed = false;
  const git: GitTransport = { ...props.git,
    fetch: (root) => gitOperation(root, () => props.git.fetch(root)),
    pull: (root) => gitOperation(root, () => props.git.pull(root)),
    push: (root) => gitOperation(root, () => props.git.push(root)),
    checkout: (root, branch) => gitOperation(root, () => props.git.checkout(root, branch)),
    createBranch: (root, name, start, checkout) => gitOperation(root, () => props.git.createBranch(root, name, start, checkout)),
    deleteBranch: (root, name) => gitOperation(root, () => props.git.deleteBranch(root, name)),
    addWorktree: (root, path, branch) => gitOperation(root, () => props.git.addWorktree(root, path, branch)),
    removeWorktree: (root, path) => gitOperation(root, () => props.git.removeWorktree(root, path)),
    lockWorktree: (root, path) => gitOperation(root, () => props.git.lockWorktree(root, path)),
    unlockWorktree: (root, path) => gitOperation(root, () => props.git.unlockWorktree(root, path)),
    pruneWorktrees: (root) => gitOperation(root, () => props.git.pruneWorktrees(root)),
  };
  let generation = 0;
  let scanId: number | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let lastGroup = "";
  let lastDirectory = "";
  const cwd = () => props.workspace.group(props.workspace.workspaceKey())?.cwd ?? "~";
  const root = () => selected() || cwd();
  function cancelScan() {
    if (timer !== undefined) clearTimeout(timer);
    timer = undefined;
    if (scanId !== null) props.git.cancelDiscovery?.(scanId);
    scanId = null;
  }
  async function discover(directory: string) {
    cancelScan();
    const current = ++generation;
    loading.set(!selected());
    scanning.set(true);
    error.set("");
    const apply = (result: { paths: string[]; error?: string | null; done: boolean }) => {
      paths.set(result.paths);
      if (!selected() && result.paths.length) selected.set(result.paths[0]);
      if (result.done && !result.paths.includes(selected())) selected.set(result.paths[0] ?? "");
      error.set(result.error ?? "");
      if (result.paths.length || result.done) loading.set(false);
      if (result.done) scanning.set(false);
    };
    try {
      if (props.git.startDiscovery && props.git.pollDiscovery) {
        scanId = props.git.startDiscovery(directory);
        const poll = () => {
          if (current !== generation || scanId === null) return;
          try {
            const result = props.git.pollDiscovery!(scanId);
            apply(result);
            if (result.done) scanId = null;
            else timer = setTimeout(poll, 100);
          } catch (reason) { fail(reason); }
        };
        poll();
      } else {
        const result = props.git.discover ? await props.git.discover(directory) : { paths: [directory], error: null };
        if (current === generation) apply({ ...result, done: true });
      }
    } catch (reason) { fail(reason); }
    function fail(reason: unknown) {
      if (current !== generation) return;
      cancelScan();
      loading.set(false);
      scanning.set(false);
      error.set(reason instanceof Error ? reason.message : String(reason));
    }
  }
  effect(() => {
    const directory = cwd();
    const group = props.workspace.workspaceKey();
    if (group === lastGroup && directory === lastDirectory) return;
    lastGroup = group;
    lastDirectory = directory;
    untracked(() => {
      selected.set("");
      fetchError.set("");
      paths.set([]);
      void discover(directory);
    });
  });
  if (!props.preview) scheduleGitTasks({
    scan: () => { if (!scanning()) void discover(cwd()); },
    refresh: () => refreshTick.update(value => value + 1),
    fetch: () => {
      const path = selected();
      if (!path || gitOperationBusy(path)) return;
      void git.fetch(path).then(() => {
        if (!disposed && selected() === path) {
          fetchError.set("");
          refreshTick.update(value => value + 1);
        }
      }).catch(reason => {
        if (!disposed && selected() === path) fetchError.set(reason instanceof Error ? reason.message : String(reason));
      });
    },
  });
  cleanup(() => { disposed = true; generation++; cancelScan(); });
  return <View class="min-h-0 flex-1">
    <View class="shrink-0 flex-row items-center gap-2 border-b border-border px-3 py-1">
      <Type role="label" tone="muted-foreground">Repository</Type>
      <Popover open={open()} onOpenChange={({ open: value }) => open.set(value)} placement="bottom" align="start"
        popupClass="rounded-(--radius-overlay) border border-border bg-popover p-2"
        trigger={<Button testID="git-repository-selector" accessibilityLabel="Select Git repository"
          disabled={paths().length < 2} class="min-w-0 rounded-(--radius-control) px-2 py-1 hover:bg-control-hover">
          <Type role="label" lines={1}>{loading() ? "Searching…" : selected() || "No repository found"}</Type>
        </Button>}>
        <View class="gap-1">
          <Each of={paths()}>{(path) => <Button testID={`git-repository-${path()}`} accessibilityLabel={`Select ${path()}`}
            selected={selected() === path()} onPress={() => { selected.set(path()); fetchError.set(""); open.set(false); }}
            class="rounded-(--radius-control) px-2 py-1 hover:bg-control-hover">
            <Type role="label" lines={1}>{path()}</Type>
          </Button>}</Each>
        </View>
      </Popover>
      <If when={scanning() && !loading()}><Type role="caption" tone="muted-foreground">Searching…</Type></If>
      <Button testID="git-discover-refresh" accessibilityLabel="Rescan repositories" onPress={() => void discover(cwd())}
        class="rounded-(--radius-control) px-2 py-1 hover:bg-control-hover"><Type role="label">Rescan</Type></Button>
    </View>
    <If when={!!fetchError()}><Type role="caption" tone="muted-foreground" class="px-3">{`Automatic fetch failed: ${fetchError()}`}</Type></If>
    <If when={!!error()}><Type role="caption" tone="muted-foreground" class="px-3">{error()}</Type></If>
    <If when={!loading()}>
      <If when={props.workspace.activeGitView() === "changes"}><GitPanel workspace={props.workspace} preview={props.preview} git={git} root={root} refreshTick={refreshTick} rescan={() => void discover(cwd())} /></If>
      <If when={props.workspace.activeGitView() === "history"}><HistoryPanel workspace={props.workspace} preview={props.preview} git={git} root={root} refreshTick={refreshTick} rescan={() => void discover(cwd())} notify={props.notify} /></If>
    </If>
  </View>;
}
