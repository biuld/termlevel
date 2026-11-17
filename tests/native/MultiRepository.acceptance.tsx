import { expect, open, test } from "nsh/test";
test("native repository catalog discovers containers and enclosing roots and restores history", async () => {
  const app = await open("./tests/workspace/fixtures/MultiRepositoryNativeFixture.tsx", { platform: "macos", width: 1300, height: 800, colorScheme: "dark" });
  async function choose(name: string) {
    await app.press("id:git-repository-selector");
    await app.waitFor(`text:${name}`);
    const path = app.texts().find(text => text.endsWith(`/multi-repository-repos/${name}`))!;
    expect(!!path).toBe(true);
    await app.press(`id:git-repository-${path}`);
  }
  try {
    await app.waitFor("id:git-repository-selector");
    await choose("alpha"); await app.waitFor("text:alpha commit 600");
    await choose("beta"); await app.waitFor("text:beta commit 600");
    await choose("alpha"); await app.waitFor("text:alpha commit 600");
    await app.waitFor("text:Logs 3 · Resumes 1");
    await app.scroll("id:history-commits", 6000);
    await app.press("id:history-load-more");
    await app.waitFor("text:Logs 4 · Resumes 1");
    await app.screenshot("artifacts/multi-repository-native-restored.png");
    await app.press("id:repo-inside"); await app.waitFor("text:alpha commit 600");
    await app.press("id:repo-container");
    await app.press("id:git-repository-selector"); await app.waitFor("text:worktree");
    await app.hover("id:git-discover-refresh");
    await app.screenshot("artifacts/multi-repository-native-picker.png");
    await app.type("id:git-repository-search", "alpha-linked");
    await app.waitFor("text:alpha-linked");
    await app.screenshot("artifacts/multi-repository-native-filtered.png");
    await app.press("id:git-add-repository-open");
    await app.waitFor("id:git-add-repository-path");
    await app.screenshot("artifacts/multi-repository-native-add.png");
  } finally { app.dispose(); }
}, 60000);
