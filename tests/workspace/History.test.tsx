import { expect, open, test } from "nsh/test";

const options = {
  platform: "macos" as const,
  width: 1200,
  height: 780,
  colorScheme: "dark" as const,
  modules: { "termlevel/native": { "Runtime.previewMode": true } },
};

test("the history view renders branches, worktrees, commits and a patch", async () => {
  const app = await open("./fixtures/HistoryStatesFixture.tsx", options);
  await app.waitFor("id:dashboard");
  await app.press("id:dashboard-view-history");
  await app.waitFor("id:history-panel");
  await app.waitUntil(() => app.texts().some((text) => text === "Merge feature/history"));
  await app.waitUntil(() => app.texts().some((text) => text === "Initial commit"));
  expect(app.texts().some((text) => text === "main")).toBe(true);
  expect(app.texts().some((text) => text === "feature/history")).toBe(true);
  await app.waitUntil(() => app.texts().some((text) => text.includes("HistoryPanel.tsx")));
  await app.press("text:Group commits by lane");
  await app.waitUntil(() => app.texts().some((text) => text.includes("fixture for ccccccc")));
  await app.waitUntil(() => app.texts().some((text) => text.includes("import { View }")));
  await app.press("id:history-file-src-git-rs");
  await app.waitUntil(() => app.texts().some((text) => text.includes("No text diff for this file")));
  await app.press("id:history-file-ui-dashboard-HistoryPanel-tsx");
  await app.waitUntil(() => app.texts().some((text) => text.includes("import { DataTable }")));
  await app.setViewport(700, 700);
  await app.waitFor("id:history-detail-narrow");
  expect(app.shows("id:history-detail-wide")).toBe(false);
  expect(app.texts().some((text) => text === "Tyler Chan")).toBe(false);
  expect(app.texts().some((text) => text === "Hash")).toBe(false);
  await app.setViewport(1600, 900);
  await app.waitFor("id:history-detail-wide");
  expect(app.texts().some((text) => text === "Tyler Chan")).toBe(true);
  expect(app.texts().some((text) => text === "Hash")).toBe(true);

  await app.press("id:history-tab-worktrees");
  await app.waitUntil(() => app.texts().some((text) => text === "feature/history"));
  await app.press("id:history-tab-branches");
  await app.waitFor("id:history-checkout");

  await app.press("id:history-new-branch");
  await app.waitFor("id:history-new-branch-name");
  await app.type("id:history-new-branch-name", "feature/new");
  await app.press("id:history-new-branch-checkout");
  await app.press("id:history-create-branch");
  await app.waitUntil(() => app.texts().some((text) => text === "feature/new"));
  expect(app.shows("id:history-new-branch-name")).toBe(false);
  await app.press("id:history-delete-branch");
  await app.waitFor("id:history-delete-branch-confirm");
  await app.press("id:history-delete-branch-confirm");
  await app.waitUntil(() => !app.texts().some((text) => text === "feature/new"));

  await app.press("id:history-new-branch");
  await app.waitFor("id:history-new-branch-name");
  await app.type("id:history-new-branch-name", "feature/wt");
  await app.press("id:history-new-branch-checkout");
  await app.press("id:history-create-branch");
  await app.waitUntil(() => app.texts().some((text) => text === "feature/wt"));
  await app.press("id:history-tab-worktrees");
  await app.press("id:history-new-worktree");
  await app.waitFor("id:history-new-worktree-path");
  await app.press("id:history-create-worktree");
  await app.waitUntil(() => app.texts().some((text) => text === "feature-wt"));
  await app.press("id:history-worktree-remove");
  await app.waitFor("id:history-remove-worktree");
  await app.press("id:history-remove-worktree");
  await app.waitUntil(() => !app.texts().some((text) => text === "feature-wt"));
  app.dispose();
});
