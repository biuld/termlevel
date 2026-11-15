import { open, test } from "nsh/test";

test("the overview reads the real repository through the system git CLI", async () => {
  const app = await open("./tests/workspace/fixtures/GitLiveFixture.tsx", {
    platform: "macos", width: 1200, height: 780, colorScheme: "dark",
  });
  try {
    await app.waitFor("id:dashboard");
    await app.waitUntil(() =>
      app.texts().some((text) => text === "Changed files" || text === "Working tree clean"));
    await app.wait(600);
    await app.screenshot("artifacts/dashboard-git-live.png");
    await app.press("id:dashboard-view-history");
    await app.waitFor("id:history-panel");
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
  } finally {
    app.dispose();
  }
});
