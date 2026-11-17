import { expect, open, test } from "nsh/test";

test("Git scale native: twenty thousand refs and long history remain responsive", async () => {
  const app = await open("./tests/workspace/fixtures/GitScaleNativeFixture.tsx", {
    platform: "macos", width: 1400, height: 850, colorScheme: "dark",
  });
  try {
    await app.waitUntil(() => app.texts().some(text => text.includes("Refs 20002")), { timeout: 20000 });
    await app.waitFor("text:Scale commit 10000");
    await app.screenshot("artifacts/git-scale-native-loaded.png");
    const first = app.texts().find(text => text.startsWith("Refs 20002"))!;
    console.log(`Initial load: ${first}`);
    expect(Number(/Max gap (\d+)/.exec(first)![1]) < 500).toBe(true);
    await app.press("id:scale-native-reset");
    await app.press("id:scale-native-refresh");
    await app.waitUntil(() => app.texts().some(text => text.includes("Unchanged 1")), { timeout: 20000 });
    const refreshed = app.texts().find(text => text.startsWith("Refs 20002"))!;
    console.log(`Unchanged refresh: ${refreshed}`);
    expect(refreshed.includes("Logs 1")).toBe(true);
    expect(Number(/Max gap (\d+)/.exec(refreshed)![1]) < 500).toBe(true);
    await app.scroll("id:history-commits", 6000);
    await app.waitFor("id:history-load-more");
    await app.press("id:history-load-more");
    await app.scroll("id:history-commits", 400);
    await app.waitFor("text:Scale commit 9800");
    await app.screenshot("artifacts/git-scale-native-page-two.png");
    await app.press("id:history-search-open");
    await app.waitFor("id:history-search");
    await app.type("id:history-search", "19099");
    await app.waitFor("text:1/400");
    await app.waitFor("text:Scale commit 9901");
    await app.screenshot("artifacts/git-scale-native-ref-search.png");
  } finally { app.dispose(); }
}, 60000);

test("Git scale native: long labels and search fit wide and narrow panes", async () => {
  const app = await open("./tests/workspace/fixtures/GitScaleFixture.tsx", {
    platform: "macos", width: 1400, height: 850, colorScheme: "dark",
  });
  try {
    await app.waitFor("text:Scale commit 0");
    await app.screenshot("artifacts/git-long-refs-wide.png");
    await app.hover("id:history-commit-row-0000000 >> label:HEAD -> feature/history-rendering-and-large-repository-performance/keep-the-distinguishing-suffix");
    await app.wait(600);
    await app.screenshot("artifacts/git-long-ref-tooltip.png");
    await app.press("id:history-search-open");
    await app.waitFor("id:history-search");
    await app.screenshot("artifacts/git-long-refs-search-open.png");
    await app.type("id:history-search", "Scale commit 3999");
    await app.screenshot("artifacts/git-long-refs-input.png");
    await app.waitFor("text:1/4000");
    await app.waitFor("text:Connections are hidden while filtering commits.");
    expect(app.shows("id:history-commit-0000000")).toBe(false);
    await app.screenshot("artifacts/git-long-refs-search.png");
    await app.press("id:history-search-close");
    await app.setViewport(800, 700);
    await app.waitFor("id:history-detail-narrow");
    await app.screenshot("artifacts/git-long-refs-narrow.png");
  } finally { app.dispose(); }
}, 60000);
