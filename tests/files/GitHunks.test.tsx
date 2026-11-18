import { expect, test } from "nsh/test";
import { diffHunks } from "../../ui/files/git-hunks";
import { repoPath } from "../../ui/files/git-status";

test("a replacement, an insertion, and a trailing deletion keep working-tree lines", () => {
  const patch = [
    "diff --git a/a.ts b/a.ts",
    "--- a/a.ts",
    "+++ b/a.ts",
    "@@ -1,5 +1,5 @@",
    " same",
    "-old",
    "+new",
    "+extra",
    " same2",
    "-tail",
    "",
  ].join("\n");
  expect(diffHunks(patch)).toEqual({
    spans: [{ kind: "edit", start: 1, end: 3 }],
    deletions: [
      { line: 1, column: 0 },
      { line: 3, column: 1 },
    ],
  });
});

test("an insertion and a deletion at the start stay on their boundaries", () => {
  const added = ["@@ -1,2 +1,3 @@", " same", "+added", " same2"].join("\n");
  expect(diffHunks(added).spans).toEqual([{ kind: "add", start: 1, end: 2 }]);
  expect(diffHunks(added).deletions).toEqual([]);

  const removed = ["@@ -1,2 +1,1 @@", "-gone", " keep"].join("\n");
  expect(diffHunks(removed)).toEqual({ spans: [], deletions: [{ line: 0, column: 0 }] });
});

test("a new file is entirely added and a binary diff has no lines", () => {
  const patch = ["--- /dev/null", "+++ b/new.ts", "@@ -0,0 +1,2 @@", "+one", "+two"].join("\n");
  expect(diffHunks(patch).spans).toEqual([{ kind: "add", start: 0, end: 2 }]);
  expect(diffHunks("Binary files a/pic and b/pic differ")).toEqual({ spans: [], deletions: [] });
});

test("a group file path joins the repository prefix", () => {
  expect(repoPath("app", "src/main.ts")).toBe("app/src/main.ts");
  expect(repoPath("", "main.ts")).toBe("main.ts");
  expect(repoPath(null, "main.ts")).toBeNull();
});
