import { expect, test } from "nsh/test";
import { parseSnapshot } from "../../ui/workspace/workspace-snapshot";

test("a stored document is accepted, deduplicated and normalized", () => {
  const snapshot = parseSnapshot(JSON.stringify({
    version: 1,
    groups: [
      { id: "group-1", title: "termlevel", cwd: "/work", color: "purple", collapsed: true },
      { id: "group-1", title: "duplicate", cwd: "/other", color: "red", collapsed: false },
      { id: "group-2", cwd: "/play", color: "not-a-color" },
    ],
    tabs: [
      { id: "old-a", title: "A", cwd: "/work", groupId: "group-1", pinned: true, customTitle: true,
        progress: { state: "set", percent: 42 }, activity: { state: "blocked" }, status: { state: "blocked" }, statusRevision: 17, shellPhase: "prompt" },
      { id: "old-b", title: "B", cwd: "/work", groupId: "missing", pinned: true, customTitle: false },
      { id: "old-a", title: "duplicate", cwd: "/", groupId: null, pinned: false, customTitle: false },
    ],
    layouts: {
      "group-1": {
        root: {
          kind: "split", id: "split-1", axis: "row", ratio: 2,
          first: { kind: "pane", id: "pane-1", sessionId: "old-a" },
          second: { kind: "pane", id: "pane-2", sessionId: "old-b" },
        },
        focusedPaneId: "pane-2", zoomed: true,
      },
      bad: { root: { kind: "pane", id: "pane-x", sessionId: 5 }, focusedPaneId: null, zoomed: false },
    },
    activeTabId: "old-b",
    workspaceKey: "group-1",
    expanded: false,
    sidebarWidth: 240.5,
    fontSize: "large",
    scrollback: 20000,
    appearance: "dark",
    interfaceFont: "Georgia",
    monoFont: 3,
  }));

  expect(snapshot).not.toBe(null);
  expect(snapshot!.version).toBe(1);
  expect(snapshot!.groups.length).toBe(2);
  expect(snapshot!.groups[1].title).toBe("group-2");
  expect(snapshot!.groups[1].color).toBe("purple");
  expect(snapshot!.groups[1].collapsed).toBe(false);
  expect(snapshot!.tabs.length).toBe(2);
  // A pinned tab that names a group belongs to that group.
  expect(snapshot!.tabs[0].groupId).toBe("group-1");
  expect(snapshot!.tabs[0].pinned).toBe(false);
  // An unknown group is not a pin: the tab returns to the ungrouped section.
  expect(snapshot!.tabs[1].groupId).toBe(null);
  expect(snapshot!.tabs[1].pinned).toBe(true);
  // OSC 9;4 / 7501 are runtime metadata; unknown persisted keys are dropped.
  expect("progress" in snapshot!.tabs[0]).toBe(false);
  expect("activity" in snapshot!.tabs[0]).toBe(false);
  expect("status" in snapshot!.tabs[0]).toBe(false);
  expect("statusRevision" in snapshot!.tabs[0]).toBe(false);
  expect("shellPhase" in snapshot!.tabs[0]).toBe(false);
  expect(snapshot!.layouts["group-1"].focusedPaneId).toBe("pane-2");
  expect(snapshot!.layouts["group-1"].zoomed).toBe(true);
  expect((snapshot!.layouts["group-1"].root as { ratio: number }).ratio).toBe(1);
  expect(snapshot!.layouts.bad).toBe(undefined);
  expect(snapshot!.activeTabId).toBe("old-b");
  expect(snapshot!.expanded).toBe(false);
  expect(snapshot!.sidebarWidth).toBe(240.5);
  expect(snapshot!.fontSize).toBe(null);
  expect(snapshot!.scrollback).toBe(20000);
  expect(snapshot!.appearance).toBe("dark");
  expect(snapshot!.interfaceFont).toBe("Georgia");
  expect(snapshot!.monoFont).toBe(null);
});

test("an empty versioned document is valid, and broken or newer ones are ignored", () => {
  const empty = parseSnapshot('{"version":1}');
  expect(empty).not.toBe(null);
  expect(empty!.groups).toEqual([]);
  expect(empty!.tabs).toEqual([]);
  expect(empty!.layouts).toEqual({});
  expect(empty!.activeTabId).toBe(null);
  expect(empty!.workspaceKey).toBe("");
  expect(empty!.expanded).toBe(true);
  expect(empty!.appearance).toBe("system");

  expect(parseSnapshot("{not json")).toBe(null);
  expect(parseSnapshot("[]")).toBe(null);
  expect(parseSnapshot(JSON.stringify({ version: 2, groups: [] }))).toBe(null);
});

test("a layout that would draw one terminal twice is dropped", () => {
  const snapshot = parseSnapshot(JSON.stringify({
    version: 1,
    layouts: {
      "group-1": {
        root: {
          kind: "split", id: "split-1", axis: "row", ratio: 0.5,
          first: { kind: "pane", id: "pane-1", sessionId: "tab-a" },
          second: { kind: "pane", id: "pane-1", sessionId: "tab-a" },
        },
        focusedPaneId: "pane-1", zoomed: false,
      },
      "group-2": {
        root: {
          kind: "split", id: "split-2", axis: "column", ratio: 0.5,
          first: { kind: "pane", id: "pane-2", sessionId: "tab-a" },
          second: { kind: "pane", id: "pane-3", sessionId: "tab-a" },
        },
        focusedPaneId: "pane-2", zoomed: false,
      },
    },
  }));
  expect(snapshot).not.toBe(null);
  expect(snapshot!.layouts["group-1"]).toBe(undefined);
  expect(snapshot!.layouts["group-2"]).toBe(undefined);
});

test("terminal identity defaults safely for old or unknown settings", () => {
  for (const terminalIdentity of [undefined, null, "unknown", 4]) {
    expect(parseSnapshot(JSON.stringify({ version: 1, terminalIdentity }))!.terminalIdentity).toBe("termlevel");
  }
  expect(parseSnapshot(JSON.stringify({ version: 1, terminalIdentity: "ghostty" }))!.terminalIdentity).toBe("ghostty");
});
