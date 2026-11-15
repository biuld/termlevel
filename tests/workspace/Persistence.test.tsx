import { expect, open, test } from "nsh/test";

const options = {
  platform: "macos" as const,
  width: 1400,
  height: 900,
  colorScheme: "dark" as const,
  modules: { "termlevel/native": { "Runtime.previewMode": true } },
};

test("a first launch is empty and its terminal starts in the home folder", async () => {
  const app = await open("./fixtures/RestoreFixture.tsx", options);
  await app.waitFor("id:empty-state");
  expect(app.shows("id:empty-new-group")).toBe(true);
  expect(app.shows("id:empty-new-terminal")).toBe(true);
  expect(app.shows("id:sidebar-heading")).toBe(true);

  await app.press("id:empty-new-terminal");
  await app.waitFor("id:terminal-session-1");
  expect(app.shows("id:empty-state")).toBe(false);
  expect(app.shows("id:tab-session-1")).toBe(true);

  await app.waitUntil(() => !!app.storage().workspace);
  const saved = JSON.parse(app.storage().workspace);
  expect(saved.version).toBe(1);
  expect(saved.tabs.length).toBe(1);
  expect(saved.tabs[0].cwd).toBe("~");
  expect(saved.activeTabId).toBe("session-1");
  app.dispose();
});

test("the empty guide can create a home group with its first terminal", async () => {
  const app = await open("./fixtures/RestoreFixture.tsx", options);
  await app.waitFor("id:empty-state");
  await app.press("id:empty-new-group");
  await app.waitFor("id:new-group-path");
  // With no terminals open, the home folder is the only suggested path.
  await app.press("id:group-path-0");
  await app.waitFor("id:group-group-1");
  expect(app.shows("id:empty-state")).toBe(false);
  expect(app.texts()).toContain("Home");

  await app.waitUntil(() => !!app.storage().workspace);
  const saved = JSON.parse(app.storage().workspace);
  expect(saved.groups[0].title).toBe("Home");
  expect(saved.tabs.length).toBe(1);
  expect(saved.tabs[0].groupId).toBe("group-1");
  app.dispose();
});

test("a saved workspace reopens groups, pins and the split in new sessions", async () => {
  const stored = {
    version: 1,
    groups: [
      { id: "group-1", title: "termlevel", cwd: "/work", color: "purple", collapsed: false },
      { id: "group-2", title: "Playground", cwd: "/play", color: "blue", collapsed: true },
    ],
    tabs: [
      { id: "old-a", title: "UI prototype", cwd: "/work", groupId: "group-1", pinned: false, customTitle: true },
      { id: "old-b", title: "Build & preview", cwd: "/work", groupId: "group-1", pinned: false, customTitle: true },
      { id: "old-c", title: "Color test", cwd: "/play", groupId: "group-2", pinned: false, customTitle: true },
      { id: "old-pin", title: "Pinned", cwd: "~", groupId: null, pinned: true, customTitle: true },
    ],
    layouts: {
      "group-1": {
        root: {
          kind: "split", id: "split-1", axis: "row", ratio: 0.5,
          first: { kind: "pane", id: "pane-1", sessionId: "old-a" },
          second: { kind: "pane", id: "pane-2", sessionId: "old-b" },
        },
        focusedPaneId: "pane-2", zoomed: false,
      },
      "": { root: { kind: "pane", id: "pane-3", sessionId: "old-pin" }, focusedPaneId: "pane-3", zoomed: false },
    },
    activeTabId: "old-a",
    workspaceKey: "group-1",
    expanded: true,
    sidebarWidth: 240,
    fontSize: 18,
    scrollback: 20000,
    appearance: "light",
    interfaceFont: "Georgia",
    monoFont: "Menlo",
  };
  const app = await open("./fixtures/RestoreFixture.tsx", {
    ...options, storage: { workspace: JSON.stringify(stored) },
  });

  await app.waitFor("id:terminal-session-1");
  expect(app.shows("id:group-group-1")).toBe(true);
  expect(app.shows("id:group-group-2")).toBe(true);
  expect(app.shows("id:split-unit-group-1 >> id:tab-session-1")).toBe(true);
  expect(app.shows("id:split-unit-group-1 >> id:tab-session-2")).toBe(true);
  expect(app.shows("id:terminal-session-1")).toBe(true);
  expect(app.shows("id:terminal-session-2")).toBe(true);
  // The saved active tab wins over the saved pane focus.
  expect(app.shows("id:pane-session-1 >> id:session-toolbar")).toBe(true);
  // A collapsed group keeps its saved state instead of expanding for the restore.
  expect(app.shows("id:tab-session-3")).toBe(false);
  expect(app.shows("id:tab-session-4")).toBe(true);
  expect(app.texts()).toContain("UI prototype");
  expect(app.texts()).toContain("Pinned");

  await app.waitUntil(() => {
    const raw = app.storage().workspace;
    return !!raw && JSON.parse(raw).activeTabId === "session-1";
  });
  const saved = JSON.parse(app.storage().workspace);
  expect(saved.tabs.map((tab: { id: string }) => tab.id)).toEqual(["session-1", "session-2", "session-3", "session-4"]);
  expect(saved.sidebarWidth).toBe(240);
  expect(saved.fontSize).toBe(18);
  expect(saved.scrollback).toBe(20000);
  expect(saved.appearance).toBe("light");
  expect(saved.layouts["group-1"].root.first.sessionId).toBe("session-1");
  app.dispose();
});

test("a saved directory that disappeared reopens in the home folder", async () => {
  const app = await open("./fixtures/RestoreFixture.tsx", {
    ...options,
    storage: {
      workspace: JSON.stringify({
        version: 1,
        groups: [],
        tabs: [{ id: "old-a", title: "Gone", cwd: "/missing", groupId: null, pinned: false, customTitle: true }],
        layouts: {
          "": { root: { kind: "pane", id: "pane-1", sessionId: "old-a" }, focusedPaneId: "pane-1", zoomed: false },
        },
        activeTabId: "old-a",
        workspaceKey: "",
        expanded: true,
      }),
    },
  });
  // session-1 is the failed open; it is closed and session-2 starts in home.
  await app.waitFor("id:terminal-session-2");
  expect(app.shows("id:tab-session-1")).toBe(false);
  expect(app.shows("id:session-path")).toBe(true);
  expect(app.texts()).toContain("~");

  await app.waitUntil(() => {
    const raw = app.storage().workspace;
    return !!raw && JSON.parse(raw).tabs[0]?.id === "session-2";
  });
  const saved = JSON.parse(app.storage().workspace);
  expect(saved.tabs[0].cwd).toBe("~");
  expect(saved.layouts[""].root.sessionId).toBe("session-2");
  app.dispose();
});

test("corrupt storage and group-only storage both fall back to the empty guide", async () => {
  const broken = await open("./fixtures/RestoreFixture.tsx", {
    ...options, storage: { workspace: "{not json" },
  });
  await broken.waitFor("id:empty-state");
  expect(broken.shows("id:group-group-1")).toBe(false);
  broken.dispose();

  const groups = await open("./fixtures/RestoreFixture.tsx", {
    ...options,
    storage: {
      workspace: JSON.stringify({
        version: 1,
        groups: [{ id: "group-1", title: "termlevel", cwd: "/work", color: "purple", collapsed: false }],
        tabs: [],
        layouts: {},
        activeTabId: null,
        workspaceKey: "",
        expanded: true,
      }),
    },
  });
  await groups.waitFor("id:empty-state");
  expect(groups.shows("id:group-group-1")).toBe(true);
  expect(groups.shows("id:tab-session-1")).toBe(false);
  groups.dispose();
});

test("terminal identity is saved and restored through settings", async () => {
  const app = await open("./fixtures/RestoreFixture.tsx", {
    ...options,
    storage: { workspace: JSON.stringify({ version: 1, terminalIdentity: "ghostty" }) },
  });
  try {
    await app.waitFor("id:empty-state");
    await app.press("id:settings");
    await app.press("id:settings-terminal");
    expect(app.texts()).toContain("TERM_PROGRAM=ghostty");
    await app.press("id:terminal-identity-termlevel");
    await app.waitUntil(() => JSON.parse(app.storage().workspace).terminalIdentity === "termlevel");
    await app.press("id:terminal-identity-ghostty");
    await app.waitUntil(() => JSON.parse(app.storage().workspace).terminalIdentity === "ghostty");
  } finally {
    app.dispose();
  }
});

test("the working indicator is saved and restored through settings", async () => {
  const app = await open("./fixtures/RestoreFixture.tsx", {
    ...options,
    storage: { workspace: JSON.stringify({ version: 1, workingIndicator: "nope" }) },
  });
  try {
    await app.waitFor("id:empty-state");
    // An unknown stored value falls back and is rewritten by the workspace save.
    await app.waitUntil(() => JSON.parse(app.storage().workspace).workingIndicator === "bars");
    await app.press("id:settings");
    await app.press("id:settings-interface");
    await app.press("id:working-indicator-vinyl");
    await app.waitUntil(() => JSON.parse(app.storage().workspace).workingIndicator === "vinyl");
    await app.press("id:working-indicator-beat");
    await app.waitUntil(() => JSON.parse(app.storage().workspace).workingIndicator === "beat");
  } finally {
    app.dispose();
  }
});
