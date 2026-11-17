import { cleanup, effect, signal, untracked } from "nsh";

export interface RepositoryPreference { selectedRoot: string | null; manual: boolean; addedRoots: string[] }
export type RepositoryPreferences = Record<string, RepositoryPreference>;
export interface RepositorySummary { branch: string; changes: number; conflicts: number }

export function parseRepositoryPreferences(value: unknown, groupIds: Set<string>): RepositoryPreferences {
  const result: RepositoryPreferences = {};
  if (!value || typeof value !== "object" || Array.isArray(value)) return result;
  for (const [id, raw] of Object.entries(value)) {
    if (!groupIds.has(id) || !raw || typeof raw !== "object") continue;
    const entry = raw as Record<string, unknown>;
    const path = (value: unknown) => typeof value === "string" && value.trim() && value.length <= 4096 ? value.trim() : null;
    const addedRoots = Array.isArray(entry.addedRoots) ? [...new Set(entry.addedRoots.map(path).filter((path): path is string => !!path))].slice(0, 32) : [];
    const selectedRoot = path(entry.selectedRoot);
    result[id] = { selectedRoot, manual: entry.manual === true && !!selectedRoot, addedRoots };
  }
  return result;
}

/** Group preferences persist; process-scoped caches register their own release callbacks. */
export function createRepositoryPreferences(initial: RepositoryPreferences, groupIds: () => string[]) {
  const repositoryPreferences = signal(initial);
  const repositorySummaries = signal<Record<string, RepositorySummary>>({});
  const disposers = new Set<() => void>();
  const listeners = new Set<(ids: Set<string>) => void>();
  const repositoryPreference = (id: string): RepositoryPreference => repositoryPreferences()[id] ?? { selectedRoot: null, manual: false, addedRoots: [] };
  function selectRepository(id: string, root: string | null, manual = true) {
    repositoryPreferences.update(all => ({ ...all, [id]: { ...repositoryPreference(id), selectedRoot: root, manual } }));
  }
  function addRepositoryRoot(id: string, path: string) {
    const root = path.trim();
    if (!root) return;
    repositoryPreferences.update(all => ({ ...all, [id]: { ...repositoryPreference(id), addedRoots: [...new Set([...repositoryPreference(id).addedRoots, root])].slice(-32) } }));
  }
  function updateRepositorySummary(root: string, summary: RepositorySummary) {
    repositorySummaries.update(all => Object.fromEntries([...Object.entries(all).filter(([key]) => key !== root), [root, summary]].slice(-128)));
  }
  effect(() => {
    const ids = new Set(groupIds());
    untracked(() => {
      const all = repositoryPreferences();
      if (Object.keys(all).some(id => !ids.has(id))) repositoryPreferences.set(Object.fromEntries(Object.entries(all).filter(([id]) => ids.has(id))));
      for (const listener of listeners) listener(ids);
    });
  });
  cleanup(() => { for (const dispose of disposers) dispose(); disposers.clear(); listeners.clear(); });
  return {
    repositoryPreferences, repositoryPreference, selectRepository, addRepositoryRoot, repositorySummaries, updateRepositorySummary,
    registerRepositoryResources(dispose: () => void, groupsChanged: (ids: Set<string>) => void) {
      disposers.add(dispose); listeners.add(groupsChanged);
    },
  };
}
