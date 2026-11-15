import { open, test } from "nsh/test";

test("the history panel draws lanes, branches, worktrees and a commit patch", async () => {
  const app = await open("./tests/workspace/fixtures/HistoryStatesFixture.tsx", {
    platform: "macos", width: 1200, height: 780, colorScheme: "dark",
  });
  try {
    await app.waitFor("id:dashboard");
    await app.press("id:dashboard-view-history");
    await app.waitFor("id:history-panel");
    await app.waitUntil(() => app.texts().some((text) => text === "Merge feature/history"));
    await app.wait(500);
    await app.screenshot("artifacts/history-merge-graph.png");
    await app.press("text:Group commits by lane");
    await app.wait(300);
    await app.screenshot("artifacts/history-selection.png");
    await app.hover("id:history-commit-row-bbbbbbb");
    await app.wait(200);
    await app.screenshot("artifacts/history-hover.png");
    await app.hover("id:history-file-src-git-rs");
    await app.wait(200);
    await app.screenshot("artifacts/history-file-hover.png");
    await app.press("id:history-file-src-git-rs");
    await app.wait(200);
    await app.screenshot("artifacts/history-file-diff-empty.png");
    await app.press("id:history-file-ui-dashboard-HistoryPanel-tsx");
    await app.wait(200);
    await app.screenshot("artifacts/history-file-diff.png");
    await app.setViewport(1800, 1000);
    await app.wait(400);
    await app.screenshot("artifacts/history-wide.png");
    await app.setViewport(920, 680);
    await app.wait(400);
    await app.screenshot("artifacts/history-920.png");
    await app.setViewport(1200, 780);
    await app.wait(300);
    await app.press("id:history-tab-worktrees");
    await app.wait(300);
    await app.screenshot("artifacts/history-worktrees.png");
  } finally {
    app.dispose();
  }
});
