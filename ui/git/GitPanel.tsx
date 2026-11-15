import { batch, cleanup, computed, effect, lifecycle, signal, Text, token, untracked, View } from "nsh";
import { Button, DiffView, ResizablePanel, ResizablePanelGroup, Tree, type DiffLayout, type TreeEntry, type TreeItem } from "nsh/base";
import { Hint, IconButton } from "../shared/controls";
import { interfaceFont, monoFont } from "../shared/fonts";
import { Icon } from "../shared/icons";
import { Type } from "../shared/type";
import type { Workspace } from "../workspace/state";
import type { GitChange, GitDiffMode, GitRepoStatus, GitTransport } from "./git-types";

interface TreeNode {
  name: string;
  path: string;
  children: Map<string, TreeNode>;
  change?: GitChange;
}

interface ChangeTree {
  items: TreeItem[];
  folders: string[];
  files: string[];
  changes: Map<string, GitChange>;
}

/** Groups changed paths into a directory tree. */
function buildTree(changes: GitChange[]): ChangeTree {
  const root = new Map<string, TreeNode>();
  for (const change of changes) {
    const parts = change.path.split("/");
    let level = root;
    let path = "";
    for (let index = 0; index < parts.length; index++) {
      const name = parts[index];
      path = path ? `${path}/${name}` : name;
      let node = level.get(name);
      if (!node) {
        node = { name, path, children: new Map() };
        level.set(name, node);
      }
      if (index === parts.length - 1) node.change = change;
      level = node.children;
    }
  }
  const folders: string[] = [];
  const files: string[] = [];
  const byId = new Map<string, GitChange>();
  const toItems = (level: Map<string, TreeNode>): TreeItem[] => [...level.values()]
    .sort((a, b) => {
      const aFolder = a.children.size > 0;
      const bFolder = b.children.size > 0;
      if (aFolder !== bFolder) return aFolder ? -1 : 1;
      return a.name.localeCompare(b.name);
    })
    .map((node) => {
      if (node.children.size > 0) {
        const id = `dir:${node.path}`;
        folders.push(id);
        return { id, label: node.name, children: toItems(node.children) };
      }
      const id = `file:${node.path}`;
      files.push(id);
      if (node.change) byId.set(id, node.change);
      return { id, label: node.name };
    });
  return { items: toItems(root), folders, files, changes: byId };
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

function defaultMode(change: GitChange): GitDiffMode {
  if (change.untracked) return "untracked";
  if (change.conflicted) return "unstaged";
  if (change.unstaged) return "unstaged";
  return "staged";
}

function effectiveMode(change: GitChange, requested: GitDiffMode): GitDiffMode {
  if (change.untracked) return "untracked";
  if (change.conflicted) return "unstaged";
  if (requested === "staged" && change.staged) return "staged";
  if (requested === "unstaged" && change.unstaged) return "unstaged";
  return defaultMode(change);
}

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message;
  return typeof error === "string" ? error : "Git request failed.";
}

/** Changed files and the selected patch for one group. */
export function GitPanel(props: { workspace: Workspace; git: GitTransport; preview: boolean }) {
  const store = props.workspace;
  const groupId = () => store.workspaceKey();

  const status = signal<GitRepoStatus | null>(null);
  const statusError = signal("");
  const loading = signal(true);
  const refreshing = signal(false);
  const epoch = signal(0);
  const diffEpoch = signal(0);
  const selected = signal<string | null>(null);
  const expanded = signal<string[]>([]);
  const scope = signal<GitDiffMode>("unstaged");
  const diffLayout = signal<DiffLayout>("unified");
  const patch = signal("");
  const patchError = signal("");
  const patchLoading = signal(false);

  const tree = computed(() => buildTree(status()?.changes ?? []));
  const change = () => tree().changes.get(selected() ?? "");
  const selectedKey = computed(() => {
    const item = tree().changes.get(selected() ?? "");
    return item ? `${item.path}:${item.status}:${item.staged}:${item.unstaged}:${item.untracked}` : "";
  });
  const changes = () => status()?.changes ?? [];
  const failure = () => statusError() || status()?.error || "";
  const stagedCount = () => changes().filter((item) => item.staged).length;
  const changedCount = () => changes().filter((item) => item.unstaged && !item.conflicted).length;
  const untrackedCount = () => changes().filter((item) => item.untracked).length;
  const conflictCount = () => changes().filter((item) => item.conflicted).length;
  const clean = () => !!status()?.repo && changes().length === 0;

  let statusGeneration = 0;
  let diffGeneration = 0;
  let foldersInitialized = false;
  const knownFolders = new Set<string>();

  async function loadStatus() {
    const group = store.group(groupId());
    if (!group) {
      status.set(null);
      loading.set(false);
      refreshing.set(false);
      return;
    }
    const generation = ++statusGeneration;
    loading.set(true);
    refreshing.set(true);
    try {
      const next = await props.git.status(group.cwd);
      if (generation !== statusGeneration) return;
      status.set(next);
      statusError.set("");
      const nextTree = buildTree(next.changes);
      if (!selected() || !nextTree.changes.has(selected()!)) {
        selected.set(nextTree.files[0] ?? null);
      }
      if (!foldersInitialized) {
        expanded.set(nextTree.folders);
        nextTree.folders.forEach((id) => knownFolders.add(id));
        foldersInitialized = true;
      } else {
        const added = nextTree.folders.filter((id) => !knownFolders.has(id));
        if (added.length) {
          added.forEach((id) => knownFolders.add(id));
          expanded.update((all) => [...all, ...added]);
        }
      }
    } catch (error) {
      if (generation !== statusGeneration) return;
      status.set(null);
      statusError.set(errorMessage(error));
    } finally {
      if (generation === statusGeneration) {
        loading.set(false);
        refreshing.set(false);
      }
    }
  }

  effect(() => {
    store.group(groupId())?.cwd;
    epoch();
    void loadStatus();
  });

  if (!props.preview) {
    const timer = setInterval(() => {
      if (lifecycle() === "active" && !loading()) epoch.update((value) => value + 1);
    }, 3000);
    cleanup(() => clearInterval(timer));
  }

  effect(() => {
    const group = store.group(groupId());
    const key = selectedKey();
    const requested = scope();
    diffEpoch();
    if (!group || !key) {
      patch.set("");
      patchError.set("");
      return;
    }
    const item = untracked(() => tree().changes.get(selected() ?? ""));
    if (!item) return;
    const mode = effectiveMode(item, requested);
    if (mode !== requested) {
      scope.set(mode);
      return;
    }
    const generation = ++diffGeneration;
    patchLoading.set(true);
    void props.git.diff(group.cwd, item.path, mode).then((value) => {
      if (generation !== diffGeneration) return;
      patch.set(value);
      patchError.set("");
    }).catch((error) => {
      if (generation !== diffGeneration) return;
      patch.set("");
      patchError.set(errorMessage(error));
    }).finally(() => {
      if (generation === diffGeneration) patchLoading.set(false);
    });
  });

  function refresh() {
    epoch.update((value) => value + 1);
    diffEpoch.update((value) => value + 1);
  }

  function select(id: string) {
    const item = tree().changes.get(id);
    if (!item) return;
    batch(() => {
      selected.set(id);
      scope.set(defaultMode(item));
      patch.set("");
      patchError.set("");
    });
  }

  const renderItem = (entry: TreeEntry) => {
    const item = tree().changes.get(entry.id);
    const open = expanded().includes(entry.id);
    const testID = `change-${entry.id.replace(/[^a-zA-Z0-9]+/g, "-")}`;
    return <View testID={testID} class="h-(--height-tab) flex-row items-center gap-1.5"
      style={{ paddingLeft: 6 + entry.depth * 14, paddingRight: 8 }}>
      <If when={entry.folder}>
        <Icon name={open ? "down" : "chevron"} size={10} color={token("subtle-foreground")} />
        <Icon name="folder" size={13} color={token("icon")} />
      </If>
      <If when={!entry.folder}>
        <View style={{ width: 10 }} />
        <Icon name="file" size={13} color={token("icon")} />
      </If>
      <Type role="body" class="min-w-0 flex-1" lines={1}>{entry.label}</Type>
      <If when={!!item}>
        <View style={{
          width: 5, height: 5, borderRadius: 2.5,
          backgroundColor: item?.staged ? token("primary") : "transparent",
        }} />
        <Text style={{
          width: 12, textAlign: "center", fontFamily: monoFont(),
          fontSize: token("text-caption"), color: statusColor(item!.status),
        }}>{item!.status}</Text>
      </If>
    </View>;
  };

  function scopeLabel(item: GitChange) {
    if (item.untracked) return "Untracked";
    if (item.conflicted) return "Conflict";
    if (item.staged && item.unstaged) return "Staged + Unstaged";
    return item.staged ? "Staged" : "Unstaged";
  }

  return <View class="min-h-0 flex-1 flex-col">
    <View class="h-(--height-toolbar) shrink-0 flex-row items-center gap-2 border-b border-border px-3">
      <Icon name="gitBranch" size={14} color={token("icon")} />
      <If when={!!status()?.repo}>
        <Type role="body" family="mono" tone="foreground"
          tooltip={status()?.root ? <Hint>{`Repository: ${status()!.root!}`}</Hint> : undefined}>
          {status()?.detached ? `HEAD@${status()?.branch ?? ""}` : status()?.branch ?? "—"}
        </Type>
        <If when={!!status()?.upstream && store.canvasSize().width >= 760}>
          <Type role="caption" tone="subtle-foreground" class="min-w-0" lines={1}>{status()?.upstream ?? ""}</Type>
        </If>
        <If when={(status()?.ahead ?? 0) > 0 || (status()?.behind ?? 0) > 0}>
          <Type role="caption" tone="subtle-foreground">↑{status()?.ahead ?? 0} ↓{status()?.behind ?? 0}</Type>
        </If>
        <If when={!!status()?.operation}>
          <Text class="text-(length:--text-caption)" style={{ fontFamily: interfaceFont(), lineHeight: token("leading-caption"), color: token("notification-warning") }}>{status()!.operation!.toUpperCase()}</Text>
        </If>
        <View class="h-3 w-px bg-border" />
        <If when={clean()}>
          <Type role="label" tone="muted-foreground">Working tree clean</Type>
        </If>
        <If when={!clean()}>
          <Type role="label" tone="muted-foreground" class="min-w-0" lines={1}>
            {`${stagedCount()} staged · ${changedCount()} changed · ${untrackedCount()} untracked`}
          </Type>
        </If>
        <If when={conflictCount() > 0}>
          <Text class="text-(length:--text-label) font-medium" style={{ fontFamily: interfaceFont(), lineHeight: token("leading-label"), color: token("destructive") }}>{`${conflictCount()} conflicts`}</Text>
        </If>
        <If when={status()?.truncated}>
          <Type role="caption" tone="subtle-foreground">Showing the first changes</Type>
        </If>
      </If>
      <If when={!!status() && !status()!.repo && !failure()}>
        <Type role="label" tone="muted-foreground">No Git repository</Type>
      </If>
      <If when={!!failure()}>
        <Text class="text-(length:--text-label) font-medium" style={{ fontFamily: interfaceFont(), lineHeight: token("leading-label"), color: token("destructive") }}>Git unavailable</Text>
      </If>
      <If when={props.preview}>
        <Type role="caption" tone="subtle-foreground" class="ml-2" tooltip={<Hint>Git is not connected in preview mode.</Hint>}>Preview</Type>
      </If>
      <View class="ml-auto shrink-0 flex-row items-center gap-2">
        <If when={!!change()}>
          <View class="flex-row items-center gap-0.5">
            <IconButton icon="diffUnified" label="Unified diff" testID="diff-layout-unified" selected={diffLayout() === "unified"} onPress={() => diffLayout.set("unified")} />
            <IconButton icon="diffSplit" label="Side-by-side diff" testID="diff-layout-split" selected={diffLayout() === "split"} onPress={() => diffLayout.set("split")} />
          </View>
        </If>
        <If when={refreshing()}>
          <Type role="caption" tone="subtle-foreground">Refreshing…</Type>
        </If>
        <IconButton icon="refresh" label="Refresh changes" testID="dashboard-refresh" onPress={refresh} />
      </View>
    </View>

    <If when={!status() && loading()}>
      <View class="flex-1 items-center justify-center gap-2">
        <Type role="heading">Loading changes…</Type>
        <Type role="body" tone="muted-foreground">Reading this folder with Git.</Type>
      </View>
    </If>
    <If when={!!failure()}>
      <View class="flex-1 items-center justify-center gap-3 px-6">
        <View class="h-10 w-10 items-center justify-center rounded-(--radius-panel) border border-border bg-muted">
          <Icon name="alert" size={18} color={token("destructive")} />
        </View>
        <Type role="heading">Git needs attention.</Type>
        <Type role="body" tone="muted-foreground" class="text-center">{failure()}</Type>
        <IconButton icon="refresh" label="Retry Git" onPress={refresh} />
      </View>
    </If>
    <If when={!failure() && status() && !status()!.repo}>
      <View class="flex-1 items-center justify-center gap-3 px-6">
        <View class="h-10 w-10 items-center justify-center rounded-(--radius-panel) border border-border bg-muted">
          <Icon name="folder" size={18} />
        </View>
        <Type role="heading">No Git repository here.</Type>
        <Type role="body" tone="muted-foreground" class="text-center">This folder is not inside a Git work tree.</Type>
        <Type role="body" family="mono" tone="subtle-foreground" lines={1}>{store.group(groupId())?.cwd ?? ""}</Type>
      </View>
    </If>
    <If when={!failure() && !!status()?.repo}>
      <If when={clean()}>
        <View class="flex-1 items-center justify-center gap-3">
          <View class="h-10 w-10 items-center justify-center rounded-(--radius-panel) border border-border bg-muted">
            <Icon name="check" size={18} color={token("group-green")} />
          </View>
          <Type role="heading">Working tree clean.</Type>
          <Type role="body" tone="muted-foreground">There are no changes to review.</Type>
        </View>
      </If>
      <If when={!clean()}>
        <ResizablePanelGroup class="min-h-0 flex-1"
          handleClass="bg-transparent group-hover:bg-sidebar-border group-active:bg-ring"
          handleAccessibilityLabel="Resize changed files and diff">
          <ResizablePanel size={320} minSize={220} maxSize={480}>
            <View class="h-full min-h-0 flex-col border-r border-border">
              <View class="h-(--height-status) shrink-0 flex-row items-center px-3">
                <Type role="label" class="min-w-0 flex-1">Changed files</Type>
                <Type role="mono">{`${changes().length}`}</Type>
              </View>
              <Tree class="min-h-0 flex-1" listClass="py-1"
                accessibilityLabel="Changed files"
                items={tree().items} expanded={expanded()} selected={selected()}
                itemClass="mx-1 rounded-(--radius-tab) hover:bg-tab-hover selected:bg-tab-selected"
                renderItem={renderItem}
                onExpandedChange={({ expanded: next }) => expanded.set(next)}
                onSelectedChange={({ selected: id }) => select(id)} />
            </View>
          </ResizablePanel>
          <ResizablePanel minSize={360}>
            <View class="h-full min-h-0 flex-col">
              <If when={!!change()}>
                <View class="h-(--height-status) shrink-0 flex-row items-center gap-2 border-b border-border px-3">
                  <Icon name="file" size={12} color={token("icon")} />
                  <Type role="body" family="mono" tone="foreground" class="min-w-0 flex-1" lines={1}>{change()!.path}</Type>
                  <Text style={{
                    fontFamily: monoFont(), fontSize: token("text-caption"),
                    color: statusColor(change()!.status),
                  }}>{change()!.status}</Text>
                  <If when={change()!.staged && change()!.unstaged && !change()!.untracked}>
                    <View class="flex-row rounded-(--radius-tab) bg-chrome p-0.5">
                      <Each of={["unstaged", "staged"] as const}>{(mode) =>
                        <Button testID={`diff-scope-${mode()}`} accessibilityLabel={`${mode()} diff`}
                          selected={scope() === mode()} onPress={() => scope.set(mode())}
                          class="h-(--height-control) items-center justify-center rounded-(--radius-control) px-2 focus-visible:ring-2 ring-sidebar-ring"
                          classList={{ "bg-sidebar": scope() === mode() }}>
                          <Type role="label" tone={scope() === mode() ? "foreground" : "muted-foreground"}>
                            {mode() === "unstaged" ? "Unstaged" : "Staged"}
                          </Type>
                        </Button>}</Each>
                    </View>
                  </If>
                  <If when={!change()!.staged || !change()!.unstaged || change()!.untracked}>
                    <Type role="caption" tone="muted-foreground">{scopeLabel(change()!)}</Type>
                  </If>
                </View>
                <If when={!!patchError()}>
                  <View class="flex-1 items-center justify-center gap-3 px-6">
                    <Icon name="alert" size={18} color={token("destructive")} />
                    <Type role="body" tone="muted-foreground" class="text-center">{patchError()}</Type>
                  </View>
                </If>
                <If when={!patchError() && patchLoading() && !patch()}>
                  <View class="flex-1 items-center justify-center">
                    <Type role="body" tone="muted-foreground">Loading patch…</Type>
                  </View>
                </If>
                <If when={!patchError() && !patchLoading() && !patch()}>
                  <View class="flex-1 items-center justify-center gap-2">
                    <Type role="heading">No text diff.</Type>
                    <Type role="body" tone="muted-foreground">Open this path in the terminal to inspect it.</Type>
                  </View>
                </If>
                <If when={!patchError() && !!patch()}>
                  <Each of={[`${selected() ?? ""}:${scope()}:${diffLayout()}`]} key={(value) => value}>{() =>
                    <DiffView testID={`changes-diff-${diffLayout()}`} patch={patch()} layout={diffLayout()}
                      softWrap contextLines={3}
                      fontFamily={monoFont()} fontSize={token("text-body")} lineHeight={token("leading-body") + 4}
                      class="min-h-0 flex-1" />
                  }</Each>
                </If>
              </If>
              <If when={!change()}>
                <View class="flex-1 items-center justify-center gap-3">
                  <View class="h-10 w-10 items-center justify-center rounded-(--radius-panel) border border-border bg-muted">
                    <Icon name="gitBranch" size={18} />
                  </View>
                  <Type role="heading">Select a changed file.</Type>
                  <Type role="body" tone="muted-foreground">Its patch appears here.</Type>
                </View>
              </If>
            </View>
          </ResizablePanel>
        </ResizablePanelGroup>
      </If>
    </If>
  </View>;
}
