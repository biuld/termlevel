import { expect, test } from "nsh/test";
import { gitOverlay, gitScope, groupFilePath, parentDirectories } from "../../ui/files/git-status";
import { buildFileTree, rowTitle } from "../../ui/files/tree";
import type { GitChange } from "../../ui/git/git-types";
import type { DirectoryListing } from "../../ui/files/types";

function change(path: string, status: string, staged = false): GitChange {
  return {
    path, status, staged, unstaged: !staged, untracked: status === "?", conflicted: status === "U", kind: "ordinary",
  };
}

const listing: DirectoryListing = {
  entries: [
    { name: "src", kind: "directory", hidden: false },
    { name: "readme.md", kind: "file", hidden: false },
  ],
  truncated: false,
  unreadable: null,
};

test("git paths inside the group become file marks", () => {
  const overlay = gitOverlay(
    [change("app/src/main.ts", "M", true), change("app/src/gone.ts", "D"), change("other/skip.ts", "?")],
    "/repo",
    "/repo/app",
  );
  expect(overlay.files.get("src/main.ts")).toEqual({
    status: "M", staged: true, unstaged: false, untracked: false, conflicted: false, deleted: false,
  });
  expect(overlay.files.get("src/gone.ts")?.deleted).toBe(true);
  expect(overlay.files.has("other/skip.ts")).toBe(false);
  expect(overlay.prefix).toBe("app");
  expect([...overlay.folders]).toEqual(["src"]);
  expect(overlay.deleted.get("src")).toEqual(["gone.ts"]);
});

test("the open file uses the same scope words as Changes", () => {
  const overlay = gitOverlay([
    change("a.ts", "M", true),
    { ...change("b.ts", "M"), staged: true, unstaged: true },
    change("c.ts", "?"),
    change("d.ts", "U"),
  ], "/repo", "/repo");
  expect(gitScope(overlay.files.get("a.ts")!)).toBe("Staged");
  expect(gitScope(overlay.files.get("b.ts")!)).toBe("Staged + Unstaged");
  expect(gitScope(overlay.files.get("c.ts")!)).toBe("Untracked");
  expect(gitScope(overlay.files.get("d.ts")!)).toBe("Conflict");
});

test("a change path maps onto the group directory", () => {
  expect(groupFilePath("/repo", "/repo", "src/app.rs")).toBe("src/app.rs");
  expect(groupFilePath("/Users/test/repo", "~/repo", "src/app.rs", "/Users/test")).toBe("src/app.rs");
  expect(groupFilePath("/repo", "/repo/app", "app/src/main.ts")).toBe("src/main.ts");
  expect(groupFilePath("/repo/app", "/repo", "src/main.ts")).toBe("app/src/main.ts");
  expect(groupFilePath("/private/tmp/repo", "/tmp/repo", "note.txt")).toBe("note.txt");
  expect(groupFilePath("/repo", "/repo", "other/skip.ts")).toBe("other/skip.ts");
  expect(groupFilePath("/repo", "/elsewhere", "src/app.rs")).toBe(null);
  expect(groupFilePath("/repo", "/repo/app", "other/skip.ts")).toBe(null);
  expect(parentDirectories("src/files/tree.ts")).toEqual(["src", "src/files"]);
});

test("a canonical tmp path matches the private prefix", () => {
  const overlay = gitOverlay([change("note.txt", "A")], "/private/tmp/repo", "/tmp/repo");
  expect(overlay.files.get("note.txt")?.status).toBe("A");
});

test("a deleted file stays visible under its folder", () => {
  const overlay = gitOverlay([change("src/gone.ts", "D"), change("readme.md", "?")], "/repo", "/repo");
  const tree = buildFileTree(
    { "": listing, src: { entries: [], truncated: false, unreadable: null } },
    ["src"],
    false,
    {},
    overlay,
  );
  expect(tree.items.map((item) => rowTitle(item.label))).toEqual(["src", "readme.md"]);
  expect(tree.items[0].label.endsWith("\u200b+")).toBe(true);
  expect(tree.items[1].label.includes("?")).toBe(true);
  expect(tree.items[0].children?.map((item) => rowTitle(item.label))).toEqual(["gone.ts"]);
});
