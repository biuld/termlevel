import { expect, test } from "nsh/test";
import { buildFileTree, rememberExpanded } from "../../ui/files/tree";
import type { DirectoryListing } from "../../ui/files/types";

const root: DirectoryListing = {
  entries: [
    { name: "src", kind: "directory", hidden: false },
    { name: ".env", kind: "file", hidden: true },
    { name: "readme.md", kind: "file", hidden: false },
  ],
  truncated: false,
  unreadable: null,
};

test("a directory keeps a placeholder until it is expanded", () => {
  const closed = buildFileTree({ "": root }, [], false);
  expect(closed.items.map((item) => item.label)).toEqual(["src", "readme.md"]);
  expect(closed.items[0].children?.[0].label).toBe("Loading…");
  const open = buildFileTree({ "": root, src: { entries: [], truncated: false, unreadable: null } }, ["src"], false);
  expect(open.items[0].children?.[0].label).toBe("Empty folder");
});

test("hidden files stay out until the switch is on", () => {
  const shown = buildFileTree({ "": root }, [], true);
  expect(shown.items.map((item) => item.label)).toEqual(["src", ".env", "readme.md"]);
});

test("a watch failure is a row on that folder", () => {
  const open = buildFileTree(
    { "": root, src: { entries: [], truncated: false, unreadable: null } },
    ["src"],
    false,
    { src: "This folder cannot be watched." },
  );
  expect(open.items[0].children?.map((item) => item.label)).toEqual([
    "This folder cannot be watched.",
    "Empty folder",
  ]);
  expect(open.items[0].children?.[0].disabled).toBe(true);
});

test("expanded paths keep the newest ones when the cap is reached", () => {
  const current = Array.from({ length: 64 }, (_, index) => `dir-${index}`);
  const next = rememberExpanded(current, [...current, "fresh"]);
  expect(next).toHaveLength(64);
  expect(next[0]).toBe("fresh");
  expect(next.includes("dir-63")).toBe(false);
});
