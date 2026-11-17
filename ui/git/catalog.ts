import { batch, cleanup, effect, signal, untracked } from "nsh";
import type { GitDiscoveryPage, GitRepository, GitTransport } from "./git-types";
import type { Workspace } from "../workspace/state";

function fromPath(root: string, index: number): GitRepository {
  return { id: index + 1, root, label: root.split(/[\\/]/).pop() || root, gitDir: `${root}/.git`, commonDir: root, kind: "main" };
}

/** Bounded native pages, plus compatibility for deterministic injected discovery transports. */
export function createRepositoryCatalog(workspace: Workspace, git: GitTransport) {
  const repositories = signal<GitRepository[]>([]);
  const scanning = signal(true);
  const error = signal("");
  const preferred = signal<string | null>(null);
  const contextRoots = signal<Array<{ path: string; root: string }>>([]);
  let contextKey = "";
  let subscription: number | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let generation = 0;
  let revision = -1;
  let offset = 0;
  let building: GitRepository[] = [];
  const groupId = () => workspace.workspaceKey();
  const cwd = () => workspace.group(groupId())?.cwd ?? "~";
  const contexts = () => {
    const active = workspace.activeTab();
    const preference = workspace.repositoryPreference(groupId());
    return [...new Set([
      active?.groupId === groupId() ? active.liveCwd || active.cwd : cwd(),
      ...preference.addedRoots,
      ...(preference.manual && preference.selectedRoot ? [preference.selectedRoot] : []),
      ...workspace.tabs().filter(tab => tab.groupId === groupId()).map(tab => tab.liveCwd || tab.cwd),
    ])].slice(0, 128);
  };
  function stop() {
    generation++;
    if (timer !== undefined) clearTimeout(timer);
    if (subscription !== null) git.cancelDiscovery?.(subscription);
    subscription = null;
  }
  function apply(page: GitDiscoveryPage) {
    const records = page.repositories ?? page.paths.map(fromPath);
    if (page.revision === undefined) {
      repositories.set(records); preferred.set(page.preferredRoot ?? null);
      contextRoots.set(page.contextRoots ?? []);
      scanning.set(!page.done); error.set(page.error ?? "");
      return false;
    }
    if (page.reset || page.revision !== revision) { revision = page.revision; offset = 0; building = []; }
    building.push(...records);
    offset = building.length;
    const more = offset < (page.total ?? offset);
    if (!more) {
      batch(() => {
        repositories.set(building);
        preferred.set(page.preferredRoot ?? null);
        contextRoots.set(page.contextRoots ?? []);
        scanning.set(page.scanning ?? !page.done);
        error.set(page.error ?? "");
      });
      offset = 0; building = [];
    }
    return more;
  }
  function poll(current: number) {
    if (current !== generation || subscription === null || !git.pollDiscovery) return;
    try {
      const page = git.pollDiscovery(subscription, revision < 0 ? 0 : revision, offset);
      const changed = page.revision === undefined || page.revision !== revision || offset > 0;
      const more = changed ? apply(page) : false;
      timer = setTimeout(() => poll(current), more ? 16 : 150);
    } catch (next) { error.set(next instanceof Error ? next.message : String(next)); scanning.set(false); }
  }
  async function start() {
    stop(); revision = -1; offset = 0; building = [];
    repositories.set([]); preferred.set(null); scanning.set(true); error.set("");
    const current = generation;
    if (git.startDiscovery && git.pollDiscovery) {
      const roots = contexts(); contextKey = JSON.stringify(roots);
      subscription = git.startDiscovery(cwd(), roots);
      poll(current);
    } else {
      try {
        const result = git.discover ? await git.discover(cwd()) : { paths: [cwd()] };
        if (current === generation) apply({ ...result, done: true });
      } catch (next) { if (current === generation) { error.set(String(next)); scanning.set(false); } }
    }
  }
  function rescan() {
    if (subscription !== null && git.refreshDiscovery) { scanning.set(true); git.refreshDiscovery(subscription); }
    else void start();
  }
  effect(() => { cwd(); groupId(); untracked(() => { void start(); }); });
  effect(() => { const current = contexts(); const key = JSON.stringify(current); untracked(() => { if (subscription !== null && key !== contextKey) { contextKey = key; scanning.set(true); git.updateDiscovery?.(subscription, current); } }); });
  cleanup(stop);
  return { repositories, scanning, error, preferred, contextRoots, rescan, prepare: () => scanning.set(true) };
}
