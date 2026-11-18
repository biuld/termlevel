import { lifecycle, token } from "nsh";
import type { GitChange, GitRepoStatus } from "../git/git-types";

/** Git status of one path inside the group directory. */
export interface PathGit {
  status: string;
  staged: boolean;
  unstaged: boolean;
  untracked: boolean;
  conflicted: boolean;
  deleted: boolean;
}

/** Status marks for the file tree. Paths are relative to the group directory. */
export interface GitOverlay {
  files: ReadonlyMap<string, PathGit>;
  /** Directories that contain a change but are not themselves the change. */
  folders: ReadonlySet<string>;
  /** Deleted file names whose parent directory is still the place to show them. */
  deleted: ReadonlyMap<string, readonly string[]>;
  /** Path of the group directory inside the repository. Empty at the repo root. */
  prefix: string | null;
}

export function emptyGitOverlay(): GitOverlay {
  return { files: new Map(), folders: new Set(), deleted: new Map(), prefix: null };
}

/** Repository path of a group-relative file. Null when the group is outside the repo. */
export function repoPath(prefix: string | null, relative: string): string | null {
  if (prefix === null || !relative) return null;
  return prefix ? `${prefix}/${relative}` : relative;
}

/** Same words Changes uses beside the open file. */
export function gitScope(mark: PathGit): string {
  if (mark.untracked) return "Untracked";
  if (mark.conflicted) return "Conflict";
  if (mark.staged && mark.unstaged) return "Staged + Unstaged";
  return mark.staged ? "Staged" : "Unstaged";
}

export function statusColor(status: string) {
  switch (status) {
    case "A": return token("group-green");
    case "M": case "T": return token("notification-warning");
    case "D": case "U": return token("destructive");
    case "R": case "C": return token("primary");
    default: return token("muted-foreground");
  }
}

/** Map repository paths onto the group directory. Paths outside that directory are dropped. */
export function gitOverlay(changes: readonly GitChange[], repoRoot: string, groupPath: string): GitOverlay {
  const prefix = directoryPrefix(repoRoot, groupPath);
  if (prefix === null) return emptyGitOverlay();
  const files = new Map<string, PathGit>();
  const folders = new Set<string>();
  const deleted = new Map<string, string[]>();
  for (const change of changes) {
    const path = groupPathOf(prefix, change.path);
    if (!path) continue;
    const mark: PathGit = {
      status: change.status,
      staged: change.staged,
      unstaged: change.unstaged,
      untracked: change.untracked,
      conflicted: change.conflicted,
      deleted: change.status === "D",
    };
    files.set(path, mark);
    const parts = path.split("/");
    for (let index = 0; index < parts.length - 1; index++) {
      folders.add(parts.slice(0, index + 1).join("/"));
    }
    if (!mark.deleted) continue;
    const slash = path.lastIndexOf("/");
    const parent = slash === -1 ? "" : path.slice(0, slash);
    const name = slash === -1 ? path : path.slice(slash + 1);
    const names = deleted.get(parent);
    if (names) names.push(name);
    else deleted.set(parent, [name]);
  }
  for (const names of deleted.values()) {
    names.sort((left, right) => left.toLowerCase().localeCompare(right.toLowerCase()) || (left < right ? -1 : left > right ? 1 : 0));
  }
  return { files, folders, deleted, prefix };
}

/** Poll Git status for the file tree. A missing status call leaves the tree unmarked. */
export function startGitStatus(input: {
  status?: (root: string) => Promise<GitRepoStatus>;
  root: () => string;
  absolute: () => Promise<string>;
  onOverlay: (overlay: GitOverlay) => void;
}): { refresh: () => void; stop: () => void } {
  if (!input.status) return { refresh: () => {}, stop: () => {} };
  const status = input.status;
  let stopped = false;
  let generation = 0;
  let signature = "";
  let timer: ReturnType<typeof setTimeout> | undefined;
  const publish = (overlay: GitOverlay) => {
    const next = overlaySignature(overlay);
    if (next === signature) return;
    signature = next;
    input.onOverlay(overlay);
  };
  const load = () => {
    const token = ++generation;
    const root = input.root();
    void Promise.all([status(root), input.absolute()]).then(([repo, groupPath]) => {
      if (stopped || token !== generation || input.root() !== root) return;
      publish(!repo.repo || repo.error ? emptyGitOverlay() : gitOverlay(repo.changes, repo.root ?? "", groupPath));
    }).catch(() => {
      // Keep the last marks. The file tree stays usable.
    });
  };
  const tick = () => {
    if (stopped) return;
    if (lifecycle() === "active") load();
    timer = setTimeout(tick, 3000);
  };
  timer = setTimeout(tick, 0);
  return {
    refresh: load,
    stop: () => {
      stopped = true;
      generation++;
      if (timer !== undefined) clearTimeout(timer);
    },
  };
}

function overlaySignature(overlay: GitOverlay): string {
  const files = [...overlay.files.entries()]
    .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
    .map(([path, mark]) => `${path}:${mark.status}:${mark.staged ? 1 : 0}:${mark.unstaged ? 1 : 0}:${mark.untracked ? 1 : 0}:${mark.conflicted ? 1 : 0}:${mark.deleted ? 1 : 0}`)
    .join("\n");
  return `${files}\n${[...overlay.folders].sort().join("\n")}`;
}

function groupPathOf(prefix: string, gitPath: string): string | null {
  if (!prefix) return gitPath || null;
  if (gitPath === prefix) return null;
  if (gitPath.startsWith(`${prefix}/`)) return gitPath.slice(prefix.length + 1) || null;
  return null;
}

function directoryPrefix(repoRoot: string, groupPath: string): string | null {
  for (const repo of pathVariants(repoRoot)) {
    for (const group of pathVariants(groupPath)) {
      if (group === repo) return "";
      if (group.startsWith(`${repo}/`)) return group.slice(repo.length + 1);
    }
  }
  return null;
}

function pathVariants(path: string): string[] {
  const trimmed = path.replace(/\/+$/, "");
  const variants = new Set([trimmed]);
  const swap = (from: string, to: string) => {
    if (trimmed === from || trimmed.startsWith(`${from}/`)) variants.add(to + trimmed.slice(from.length));
  };
  swap("/tmp", "/private/tmp");
  swap("/var", "/private/var");
  swap("/private/tmp", "/tmp");
  swap("/private/var", "/var");
  return [...variants];
}
