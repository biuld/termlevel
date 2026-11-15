import { expect, open, test } from "nsh/test";

const options = {
  platform: "macos" as const,
  width: 1200,
  height: 780,
  colorScheme: "dark" as const,
  modules: { "termlevel/native": { "Runtime.previewMode": true } },
};

test("the overview renders Git changes, a clean tree, no repository and an old Git", async () => {
  const app = await open("./fixtures/GitStatesFixture.tsx", options);
  await app.waitFor("id:dashboard");
  await app.waitFor("id:change-file-src-app-rs");
  await app.press("id:change-file-src-app-rs");
  await app.waitUntil(() => app.texts().some((text) => text.includes("unstaged-diff")));
  await app.press("id:change-file-src-both-rs");
  await app.waitFor("id:diff-scope-staged");
  await app.press("id:diff-scope-staged");
  await app.waitUntil(() => app.texts().some((text) => text.includes("staged-diff")));

  await app.press("id:mode-clean");
  await app.press("id:dashboard-refresh");
  await app.waitFor("text:Working tree clean.");

  await app.press("id:mode-norepo");
  await app.press("id:dashboard-refresh");
  await app.waitFor("text:No Git repository here.");

  await app.press("id:mode-old");
  await app.press("id:dashboard-refresh");
  await app.waitFor("text:Git 2.17.1 is too old. Termlevel needs Git 2.20 or newer.");
  expect(app.shows("id:dashboard")).toBe(true);
  app.dispose();
});
