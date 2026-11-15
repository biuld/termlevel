import { open, test } from "nsh/test";

test("capture history pane divider at its resize limits", async () => {
  const app = await open("./tests/workspace/fixtures/GitActionsFixture.tsx", {
    platform: "macos", width: 1600, height: 900, colorScheme: "dark",
  });
  try {
    await app.waitFor("id:history-panel");
    await app.waitFor("id:history-detail-wide");
    await app.drag("label:Resize history panes >> nth=1", { x: -180 });
    await app.wait(300);
    await app.screenshot("artifacts/history-divider-drop.png");
    await app.drag("label:Resize history panes >> nth=1", { x: -900 });
    await app.wait(300);
    await app.screenshot("artifacts/history-divider-limit-left.png");
    await app.drag("label:Resize history panes >> nth=1", { x: 900 });
    await app.wait(300);
    await app.screenshot("artifacts/history-divider-limit-right.png");
    await app.setViewport(900, 700);
    await app.waitFor("id:history-detail-narrow");
    await app.drag("label:Resize commits and commit details", { y: -500 });
    await app.wait(300);
    await app.screenshot("artifacts/history-divider-narrow.png");
  } finally { app.dispose(); }
});

test("the history panel draws lanes, branches, worktrees and a commit patch", async () => {
  const app = await open("./tests/workspace/fixtures/HistoryStatesFixture.tsx", {
    platform: "macos", width: 1200, height: 780, colorScheme: "dark",
  });
  try {
    await app.waitFor("id:dashboard");
    await app.press("id:dashboard-view-history");
    await app.waitFor("id:history-panel");
    await app.screenshot("artifacts/history-branch-actions-all.png");
    await app.press("text:origin/main");
    await app.screenshot("artifacts/history-branch-actions-remote.png");
    await app.press("text:All branches");
    await app.press("id:history-fetch");
    await app.waitUntil(() => app.texts().some(text => text.includes("Fetch complete")));
    await app.press("id:history-pull");
    await app.waitUntil(() => app.texts().some(text => text.includes("Cannot fast-forward divergent branches.")));
    await app.press("id:history-pull");
    await app.waitUntil(() => app.texts().some(text => text.includes("Pull complete")));
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
    await app.press("id:history-file-ui-git-HistoryPanel-panel-tsx");
    await app.wait(200);
    await app.screenshot("artifacts/history-file-diff.png");
    await app.setViewport(1800, 1000);
    await app.wait(400);
    await app.screenshot("artifacts/history-wide.png");
    await app.setViewport(920, 680);
    await app.wait(400);
    await app.screenshot("artifacts/history-920.png");
    await app.setViewport(640, 680);
    await app.wait(400);
    await app.screenshot("artifacts/history-fetch-pull-narrow.png");
    await app.setViewport(1200, 780);
    await app.wait(300);
    await app.press("id:history-tab-worktrees");
    await app.wait(300);
    await app.screenshot("artifacts/history-worktrees.png");
  } finally {
    app.dispose();
  }
});

test("History search and Git result toast fit the app window", async () => {
  const app = await open("./tests/workspace/fixtures/GitActionsFixture.tsx", {
    platform: "macos", width: 1100, height: 750, colorScheme: "dark",
  });
  try {
    await app.waitFor("id:history-panel");
    await app.press("id:history-fetch");
    await app.waitFor("id:notification-toast");
    await app.screenshot("artifacts/history-git-toast.png");
    await app.press("id:toast-dismiss");
    await app.waitFor("label:Push current branch to upstream");
    await app.press("id:history-push");
    await app.waitFor("id:notification-toast");
    await app.screenshot("artifacts/history-push-toast.png");
    await app.press("id:toast-dismiss");
    await app.press("id:history-search-open");
    await app.type("id:history-search", "Merge");
    await app.waitUntil(() => app.texts().some(text => /^1\//.test(text)));
    await app.screenshot("artifacts/history-search-open.png");
    await app.setViewport(640, 480);
    await app.wait(300);
    await app.screenshot("artifacts/history-search-compact.png");
    await app.press("id:history-search-close");
  } finally { app.dispose(); }
});
