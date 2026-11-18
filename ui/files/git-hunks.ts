import { effect, token } from "nsh";
import type { EditorDecoration } from "nsh/base";
import type { GitDiffMode } from "../git/git-types";
import { repoPath, type PathGit } from "./git-status";

/** Added or replaced lines in the working tree. `end` is exclusive. */
export interface DiffSpan {
  kind: "add" | "edit";
  start: number;
  end: number;
}

/** A deletion. Column 0 sits above the line. A positive column sits below it. */
export interface DiffDeletion {
  line: number;
  column: number;
}

/** Line marks from a unified diff. Line numbers follow the added side, from 0. */
export function diffHunks(patch: string): { spans: DiffSpan[]; deletions: DiffDeletion[] } {
  const spans: DiffSpan[] = [];
  const deletions: DiffDeletion[] = [];
  let newLine = 1;
  let inHunk = false;
  let blockStart = 0;
  const pending: Array<"del" | "add"> = [];

  const flush = () => {
    if (pending.length === 0) return;
    const adds = pending.filter((kind) => kind === "add").length;
    const removed = pending.length - adds;
    if (adds > 0) spans.push({ kind: removed > 0 ? "edit" : "add", start: blockStart, end: blockStart + adds });
    if (removed > 0) {
      if (adds > 0 || blockStart === 0) deletions.push({ line: blockStart, column: 0 });
      else deletions.push({ line: blockStart - 1, column: 1 });
    }
    pending.length = 0;
  };

  for (const line of patch.split("\n")) {
    if (line.startsWith("@@")) {
      flush();
      const match = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
      inHunk = match !== null;
      if (match) newLine = Number(match[1]);
      continue;
    }
    if (!inHunk || line.startsWith("\\")) continue;
    const prefix = line[0];
    if (prefix === " " || line === "") {
      if (line === "") continue;
      flush();
      newLine++;
      continue;
    }
    if (prefix === "-" || prefix === "+") {
      if (pending.length === 0) blockStart = Math.max(newLine, 1) - 1;
      pending.push(prefix === "-" ? "del" : "add");
      if (prefix === "+") newLine++;
    }
  }
  flush();
  return { spans, deletions };
}

/** Gutter bars and overview-ruler marks for one patch. */
export function diffDecorations(patch: string): EditorDecoration[] {
  const { spans, deletions } = diffHunks(patch);
  const marks: EditorDecoration[] = [];
  for (const span of spans) {
    marks.push(mark(span.start, span.end, span.kind === "add" ? token("group-green") : token("notification-warning"), false));
  }
  for (const deletion of deletions) {
    marks.push(mark(deletion.line, deletion.column, token("destructive"), true));
  }
  return marks;
}

function mark(start: number, end: number, color: string, removed: boolean): EditorDecoration {
  const range = removed
    ? { start: { line: start, column: end }, end: { line: start, column: end } }
    : { start: { line: start, column: 0 }, end: { line: end, column: 0 } };
  const style = { backgroundColor: color };
  return {
    range,
    gutter: { placement: removed ? "before" : "lines", style },
    overviewRuler: { lane: "left", style },
  };
}

/** Reload line marks from `git diff HEAD` while the buffer matches disk. */
export function watchGitHunks(input: {
  diff?: (root: string, path: string, mode: GitDiffMode) => Promise<string>;
  root: () => string;
  path: () => string | null;
  mark: () => PathGit | undefined;
  prefix: () => string | null;
  saved: () => string;
  clean: () => boolean;
  editable: () => boolean;
  onMarks: (marks: EditorDecoration[]) => void;
}): void {
  let generation = 0;
  let shown = false;
  let loaded: string | null | undefined;
  const clear = () => {
    generation++;
    if (!shown) return;
    shown = false;
    input.onMarks([]);
  };
  effect(() => {
    const path = input.path();
    const mark = input.mark();
    const editable = input.editable();
    const switched = loaded !== undefined && loaded !== path;
    loaded = path;
    if (switched) clear();
    if (!input.diff || !path || !mark || !editable || mark.conflicted || mark.deleted) {
      clear();
      return;
    }
    if (!input.clean()) return;
    const disk = input.saved();
    const gitPath = repoPath(input.prefix(), path);
    if (!gitPath) {
      clear();
      return;
    }
    const mode: GitDiffMode = mark.untracked ? "untracked" : "head";
    const root = input.root();
    const tokenId = ++generation;
    const diff = input.diff;
    void diff(root, gitPath, mode).then((patch) => {
      if (tokenId !== generation || !input.clean() || input.path() !== path || input.saved() !== disk) return;
      shown = true;
      input.onMarks(diffDecorations(patch));
    }).catch(() => {
      if (tokenId !== generation || !input.clean() || input.path() !== path) return;
      clear();
    });
  });
}
