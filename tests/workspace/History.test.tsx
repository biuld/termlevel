import { expect, open, test } from "nsh/test";

const options = {
  platform: "macos" as const,
  width: 1200,
  height: 780,
  colorScheme: "dark" as const,
  modules: { "termlevel/native": { "Runtime.previewMode": true } },
};

test("commit search opens on demand, counts loaded matches and clears on close", async () => {
  const app = await open("./fixtures/HistoryStatesFixture.tsx", options);
  await app.press("id:dashboard-view-history");
  await app.waitFor("id:history-search-open");
  expect(app.shows("id:history-search")).toBe(false);
  await app.press("id:history-search-open");
  await app.type("id:history-search", "Group commits");
  await app.waitFor("text:1/4");
  expect(app.shows("text:Initial commit")).toBe(false);
  await app.press("id:history-search-close");
  expect(app.shows("id:history-search")).toBe(false);
  await app.waitFor("text:Initial commit");
  await app.press("id:history-search-open");
  await app.keyboard.press("escape");
  expect(app.shows("id:history-search")).toBe(false);
  await app.press("id:history-search-open");
  await app.type("id:history-search", "Older match");
  await app.waitFor("id:history-load-more-empty");
  await app.press("id:history-load-more-empty");
  await app.waitFor("text:Older match");
  app.dispose();
});

test("branch actions stay in one fixed toolbar as the selected branch changes", async () => {
  const app = await open("./fixtures/HistoryStatesFixture.tsx", options);
  await app.press("id:dashboard-view-history");
  await app.waitFor("id:history-branch-actions");
  for (const id of ["history-checkout", "history-branch-worktree", "history-delete-branch"]) expect(app.shows(`id:${id}`)).toBe(true);
  await app.press("text:origin/main");
  expect(app.shows("id:history-branch-actions")).toBe(true);
  expect(app.texts()).not.toContain("remote branch");
  await app.press("text:main >> nth=0");
  expect(app.texts()).not.toContain("local branch");
  app.dispose();
});

test("branch loading keeps the history toolbar in place", async () => {
  const app = await open("./fixtures/HistoryStatesFixture.tsx", options);
  await app.press("id:dashboard-view-history");
  await app.waitFor("id:history-refresh");
  await app.press("text:origin/main");
  await app.waitFor("id:history-refreshing");
  for (const id of ["history-fetch", "history-pull", "history-push", "history-search-open", "history-refresh"]) expect(app.shows(`id:${id}`)).toBe(true);
  expect(app.texts()).not.toContain("Loading…");
  await app.waitUntil(() => !app.shows("id:history-refreshing"));
  app.dispose();
});

test("fetch and pull report results and refresh remote state after failure", async () => {
  const app = await open("./fixtures/HistoryStatesFixture.tsx", options);
  await app.press("id:dashboard-view-history");
  await app.waitFor("id:history-fetch");
  await app.press("id:history-fetch");
  await app.waitUntil(() => app.texts().some(text => text.includes("Fetch complete")));
  expect(app.texts()).toContain("↑1 ↓2");
  await app.press("id:history-pull");
  await app.waitUntil(() => app.texts().some(text => text.includes("Cannot fast-forward divergent branches.")));
  expect(app.texts()).toContain("↑1 ↓3");
  await app.press("id:history-pull");
  await app.waitUntil(() => app.texts().some(text => text.includes("Pull complete")));
  expect(app.texts()).toContain("↑1 ↓0");
  app.dispose();
});

test("push reports rejection and success through global notifications", async () => {
  const app = await open("./fixtures/HistoryStatesFixture.tsx", options);
  await app.press("id:dashboard-view-history");
  await app.waitFor("id:history-push");
  await app.press("text:origin/main");
  await app.press("id:history-push");
  await app.waitUntil(() => app.texts().some(text => text.includes("Push failed")));
  expect(app.texts().some(text => text.includes("Remote rejected non-fast-forward push."))).toBe(true);
  await app.waitFor("label:Push current branch to upstream");
  await app.press("id:history-push");
  await app.waitUntil(() => app.texts().some(text => text.includes("Push complete")));
  app.dispose();
});

test("the history view renders branches, worktrees, commits and a patch", async () => {
  const app = await open("./fixtures/HistoryStatesFixture.tsx", options);
  await app.waitFor("id:dashboard");
  await app.press("id:dashboard-view-history");
  await app.waitFor("id:history-panel");
  await app.waitUntil(() => app.texts().some((text) => text === "Merge feature/history"));
  await app.waitUntil(() => app.texts().some((text) => text === "Initial commit"));
  expect(app.texts().some((text) => text === "main")).toBe(true);
  expect(app.texts().some((text) => text === "feature/history")).toBe(true);
  await app.waitUntil(() => app.texts().some((text) => text.includes("HistoryPanel/panel.tsx")));
  await app.press("text:Group commits by lane");
  await app.waitUntil(() => app.texts().some((text) => text.includes("fixture for ccccccc")));
  await app.waitUntil(() => app.texts().some((text) => text.includes("import { View }")));
  await app.press("id:history-file-src-git-rs");
  await app.waitUntil(() => app.texts().some((text) => text.includes("No text diff for this file")));
  await app.press("id:history-file-ui-git-HistoryPanel-panel-tsx");
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
