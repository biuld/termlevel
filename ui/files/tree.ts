import type { TreeItem } from "nsh/base";
import type { GitOverlay } from "./git-status";
import type { DirectoryListing, FileEntry } from "./types";

export const MAX_EXPANDED = 64;

const PENDING = "\0pending";
const EMPTY = "\0empty";
const TRUNCATED = "\0truncated";
const ERROR = "\0error";

export function isMarker(id: string): boolean {
  return id.includes("\0");
}

/** Most recently expanded paths, capped so a deep walk cannot be persisted. */
export function rememberExpanded(current: readonly string[], next: readonly string[]): string[] {
  const wanted = next.filter((id) => !isMarker(id));
  const added = wanted.filter((id) => !current.includes(id));
  const previous = current.filter((id) => wanted.includes(id));
  const rest = wanted.filter((id) => !added.includes(id) && !previous.includes(id));
  return [...added, ...previous, ...rest].slice(0, MAX_EXPANDED);
}

function marker(dir: string, suffix: string, label: string): TreeItem {
  return { id: `${dir}/${suffix}`, label, disabled: true };
}

function childId(dir: string, name: string): string {
  return dir ? `${dir}/${name}` : name;
}

export interface FileTree {
  items: TreeItem[];
  entries: Map<string, FileEntry>;
}

/** Visible rows. Unloaded directories keep one disabled child so Tree treats them as folders. */
export function buildFileTree(
  listings: Readonly<Record<string, DirectoryListing | undefined>>,
  expanded: readonly string[],
  showHidden: boolean,
  watchNotes: Readonly<Record<string, string>> = {},
  git: GitOverlay | undefined = undefined,
): FileTree {
  const entries = new Map<string, FileEntry>();
  const annotate = (dir: string, items: TreeItem[]): TreeItem[] => {
    const note = watchNotes[dir];
    return note ? [marker(dir, "watch", note), ...items] : items;
  };
  const children = (dir: string): TreeItem[] => {
    const listing = listings[dir];
    if (!listing) return [marker(dir, PENDING, "Loading…")];
    if (listing.unreadable) return [marker(dir, ERROR, listing.unreadable)];
    const visible = listing.entries.filter((entry) => showHidden || !entry.hidden);
    const items = visible.map((entry) => {
      const id = childId(dir, entry.name);
      entries.set(id, entry);
      if (entry.kind === "directory" || entry.kind === "symlink") {
        return {
          id,
          label: entry.name,
          children: expanded.includes(id) ? children(id) : [marker(id, PENDING, "Loading…")],
        };
      }
      return { id, label: entry.name };
    });
    const present = new Set(visible.map((entry) => entry.name));
    for (const name of git?.deleted.get(dir) ?? []) {
      if (present.has(name) || name.includes("/") || name.includes("\\") || !name) continue;
      if (!showHidden && name.startsWith(".")) continue;
      items.push({ id: childId(dir, name), label: name });
    }
    if (!items.length) return annotate(dir, [marker(dir, EMPTY, "Empty folder")]);
    if (listing.truncated) items.push(marker(dir, TRUNCATED, "Showing the first 5000 entries"));
    return annotate(dir, items);
  };
  return { items: children(""), entries };
}
