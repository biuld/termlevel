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


test("Changes switches between unified and side-by-side patches without losing file or scope", async () => {
  const app = await open("./fixtures/GitStatesFixture.tsx", options);
  await app.waitFor("id:change-file-src-both-rs");
  await app.press("id:change-file-src-both-rs");
  await app.press("id:diff-scope-staged");
  await app.waitUntil(() => app.texts().some(text => text.includes("staged-diff")));
  await app.waitFor("id:changes-diff-unified");
  await app.press("id:diff-layout-split");
  await app.waitFor("id:changes-diff-split");
  expect(app.texts().some(text => text.includes("old-line"))).toBe(true);
  expect(app.texts().some(text => text.includes("staged-diff"))).toBe(true);
  await app.press("id:change-file-src-app-rs");
  await app.waitUntil(() => app.texts().some(text => text.includes("unstaged-diff")));
  expect(app.shows("id:changes-diff-split")).toBe(true);
  await app.setViewport(700, 700);
  expect(app.shows("id:diff-layout-unified")).toBe(true);
  await app.press("id:diff-layout-unified");
  await app.waitFor("id:changes-diff-unified");
  expect(app.texts().some(text => text.includes("unstaged-diff"))).toBe(true);
  app.dispose();
});
