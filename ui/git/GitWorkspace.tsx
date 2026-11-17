import { computed, effect, signal, untracked, View } from "nsh";
import { Button } from "nsh/base";
import { Type } from "../shared/type";
import type { Workspace } from "../workspace/state";
import { GitPanel } from "./GitPanel";
import { HistoryPanel } from "./HistoryPanel";
import type { GitTransport } from "./git-types";
import { scheduleGitTasks } from "./scheduler";
import { gitOperationBusy } from "./operation-gate";
import { createRepositoryCatalog } from "./catalog";
import { RepositoryPicker } from "./RepositoryPicker";
import { repositorySession } from "./repository-session";

/** The group owns selection; the catalog owns discovery; each working tree owns its read session. */
export function GitWorkspace(props: {
  workspace: Workspace; git: GitTransport; preview: boolean;
  notify: (level: "info" | "error", title: string, body: string) => void;
}) {
  const workspace = props.workspace;
  const catalog = createRepositoryCatalog(workspace, props.git);
  const refreshTick = signal(0);
  const fetchError = signal("");
  const addError = signal("");
  const pendingRoot = signal<string | null>(null);
  const groupId = () => workspace.workspaceKey();
  const preference = () => workspace.repositoryPreference(groupId());
  const selected = () => preference().selectedRoot;
  const record = computed(() => catalog.repositories().find(repo => repo.root === selected()));
  const session = computed(() => record() ? repositorySession(workspace, groupId(), record()!, props.git) : null);
  effect(() => { groupId(); untracked(() => { pendingRoot.set(null); addError.set(""); fetchError.set(""); }); });
  function choose(root: string) { workspace.selectRepository(groupId(), root, true); fetchError.set(""); addError.set(""); }
  effect(() => {
    const id = groupId();
    const preference = workspace.repositoryPreference(id);
    const repositories = catalog.repositories();
    const preferred = catalog.preferred();
    const scanning = catalog.scanning();
    untracked(() => {
      if (preference.manual) return;
      const next = preferred && repositories.some(repo => repo.root === preferred) ? preferred
        : preference.selectedRoot && repositories.some(repo => repo.root === preference.selectedRoot) ? preference.selectedRoot
        : !scanning && repositories.length === 1 ? repositories[0].root : null;
      if (next !== preference.selectedRoot && (!scanning || next)) workspace.selectRepository(id, next, false);
    });
  });
  effect(() => {
    const pending = pendingRoot();
    const resolved = catalog.contextRoots().find(context => context.path === pending);
    const scanning = catalog.scanning();
    untracked(() => {
      if (!pending) return;
      if (resolved) { choose(resolved.root); pendingRoot.set(null); }
      else if (!scanning && !catalog.error()) { addError.set("No Git working tree was found at this path."); pendingRoot.set(null); }
    });
  });
  if (!props.preview) scheduleGitTasks({
    refresh: () => refreshTick.update(value => value + 1),
    fetch: () => {
      const current = session();
      if (!current || gitOperationBusy(current.resourceKey)) return;
      const id = groupId();
      void current.git.fetch(current.root).then(() => {
        if (groupId() === id && selected() === current.root) { fetchError.set(""); refreshTick.update(value => value + 1); }
      }).catch(reason => { if (groupId() === id && selected() === current.root) fetchError.set(String(reason)); });
    },
  });
  return <View class="min-h-0 flex-1">
    <RepositoryPicker workspace={workspace} repositories={catalog.repositories} selected={selected}
      manual={() => preference().manual} scanning={catalog.scanning} error={() => addError() || catalog.error()}
      choose={choose} follow={() => workspace.selectRepository(groupId(), null, false)} rescan={catalog.rescan}
      add={path => { catalog.prepare(); addError.set(""); pendingRoot.set(path); workspace.addRepositoryRoot(groupId(), path); catalog.rescan(); }} />
    <If when={!!catalog.error()}><Type role="caption" class="px-3 py-1">{catalog.error()}</Type></If>
    <If when={!!fetchError()}><Type role="caption" class="px-3 py-1">{`Automatic fetch failed: ${fetchError()}`}</Type></If>
    <Each of={session() ? [session()!] : []} key={entry => `${entry.group}\0${entry.root}`}>
      {(entry) => <>
        <If when={workspace.activeGitView() === "changes"}><GitPanel workspace={workspace} preview={props.preview}
          git={entry().git} session={entry()} root={() => entry().root} refreshTick={refreshTick} rescan={catalog.rescan} /></If>
        <If when={workspace.activeGitView() === "history"}><HistoryPanel workspace={workspace} preview={props.preview}
          git={entry().git} session={entry()} root={() => entry().root} refreshTick={refreshTick} rescan={catalog.rescan} notify={props.notify} /></If>
      </>}
    </Each>
    <If when={!session()}>
      <View class="min-h-0 flex-1 items-center justify-center gap-3 px-6">
        <Type role="heading">{catalog.scanning() ? "Searching for repositories…" : selected() ? "Selected repository is unavailable." : catalog.repositories().length ? "Select a repository." : "No Git repositories found."}</Type>
        <Type role="body" class="text-center">{selected() ? "Your selection is kept. Rescan or choose another repository." : "Choose a repository above, follow a terminal directory, or add a folder."}</Type>
        <Button testID="git-empty-rescan" class="h-7 px-3 hover:bg-control-hover" onPress={catalog.rescan}><Type role="label">Rescan</Type></Button>
      </View>
    </If>
  </View>;
}
