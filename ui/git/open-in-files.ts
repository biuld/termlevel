import { Runtime } from "termlevel/native";
import { groupFilePath, parentDirectories } from "../files/git-status";
import { rememberExpanded } from "../files/tree";
import type { Workspace } from "../workspace/state";

/** Open a repository change in the group's Files view. False when the path is outside that directory. */
export function openChangeInFiles(store: Workspace, gitPath: string, repoRoot: string | null | undefined): boolean {
  const id = store.workspaceKey();
  const group = store.group(id);
  if (!group || !repoRoot) return false;
  const relative = groupFilePath(repoRoot, group.cwd, gitPath, Runtime.home());
  if (!relative) return false;
  const current = store.files()[id];
  const expanded = rememberExpanded(current?.expanded ?? [], [...(current?.expanded ?? []), ...parentDirectories(relative)]);
  const hidden = relative.split("/").some((part) => part.startsWith("."));
  store.openFiles(id);
  store.patchFiles(id, {
    selected: relative,
    expanded,
    ...(hidden || current?.showHidden ? { showHidden: true } : {}),
  });
  return true;
}
