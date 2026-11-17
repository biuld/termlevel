import { expect, open, test } from "nsh/test";

test("discovers sibling repositories and shares the selection with history", async () => {
  const app = await open("./fixtures/GitRepositoriesFixture.tsx", {
    platform: "macos", width: 1200, height: 780, colorScheme: "dark",
    modules: { "termlevel/native": { "Runtime.previewMode": true } },
  });
  await app.waitFor("text:branch-alpha");
  expect(app.texts().includes("Searching…")).toBe(true);
  await app.waitUntil(() => !app.texts().includes("Searching…"));
  await app.press("id:git-repository-selector");
  await app.press("id:git-repository-/work/beta");
  await app.waitFor("text:branch-beta");
  await app.press("id:show-history");
  await app.waitFor("text:branch-beta");
  await app.press("id:show-changes");
  await app.waitFor("text:branch-beta");
  expect(app.shows("id:git-repository-selector >> text:beta")).toBe(true);
  app.dispose();
});
