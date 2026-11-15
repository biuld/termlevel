import { open, test } from "nsh/test";

test("Git tabs read the real repository through the system git CLI", async () => {
  const app = await open("./tests/workspace/fixtures/GitLiveFixture.tsx", {
    platform: "macos", width: 1200, height: 780, colorScheme: "dark",
  });
  try {
    await app.waitFor("id:git-view");
    await app.waitUntil(() =>
      app.texts().some((text) => text === "Changed files" || text === "Working tree clean"));
    await app.wait(600);
    await app.screenshot("artifacts/dashboard-git-live.png");
    await app.hover("id:git-tab-group-1-changes");
    await app.screenshot("artifacts/git-tab-hover-title.png");
    await app.hover("id:close-git-group-1-changes");
    await app.screenshot("artifacts/git-tab-hover-close.png");
    await app.press("id:titlebar-git");
    await app.chooseMenuItem("History");
    await app.waitFor("id:history-panel");
    await app.press("id:toggle-sidebar");
    await app.waitFor("id:git-tab-group-1-changes");
    await app.waitFor("id:git-tab-group-1-history");
    await app.screenshot("artifacts/git-tabs-compact.png");
    await app.press("id:git-tab-group-1-changes");
    await app.press("id:git-tab-group-1-history");
    await app.press("id:workspace-ungrouped");
    await app.waitFor("id:empty-state");
    await app.hover("id:empty-state");
    await app.screenshot("artifacts/ungrouped-compact.png");
    await app.press("id:group-group-1");
    await app.waitFor("id:history-panel");
    await app.press("id:toggle-sidebar");
    await app.waitUntil(() => app.texts().some((text) => text === "All branches"));
    await app.wait(600);
    await app.screenshot("artifacts/dashboard-history-live.png");
    await app.press("id:history-tab-worktrees");
    await app.wait(400);
    await app.screenshot("artifacts/dashboard-history-worktrees.png");
    await app.setViewport(920, 680);
    await app.wait(500);
    await app.screenshot("artifacts/dashboard-history-920.png");
    await app.setViewport(1600, 900);
    await app.wait(500);
    await app.screenshot("artifacts/dashboard-history-wide.png");
    await app.press("id:history-commit-45aba59");
    await app.wait(600);
    await app.screenshot("artifacts/dashboard-history-45aba59.png");
  } finally {
    app.dispose();
  }
});


test("Changes switches diff layouts in a native window", async () => {
  const app = await open("./tests/workspace/fixtures/GitStatesFixture.tsx", {
    platform: "macos", width: 1200, height: 780, colorScheme: "dark",
  });
  try {
    await app.waitFor("id:changes-diff-unified");
    await app.press("id:diff-layout-split");
    await app.waitFor("id:changes-diff-split");
    await app.screenshot("artifacts/changes-diff-split-fixture.png");
    await app.setViewport(700, 700);
    await app.screenshot("artifacts/changes-diff-split-fixture-narrow.png");
    await app.press("id:diff-layout-unified");
    await app.waitFor("id:changes-diff-unified");
  } finally {
    app.dispose();
  }
});
