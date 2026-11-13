import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1200, height: 780, colorScheme: "dark" as const, modules: { "termlevel/native": { "Runtime.previewMode": true } } };

test("find in the preview terminal counts buffer matches", async () => {
  const app = await open("../App.tsx", options);
  await app.press("id:tab-preview");
  await app.keyboard.press("mod+f");
  await app.waitFor("id:find-query");
  await app.type("id:find-query", "preview");
  await app.waitUntil(() => app.texts().some((text) => /^\d+ of \d+$/.test(text)));
  expect(app.texts()).not.toContain("No matches");
  await app.press("id:find-next");
  await app.press("id:find-close");
  await app.waitUntil(() => !app.shows("id:find-in-terminal"));
  expect(app.shows("id:titlebar-new-terminal")).toBe(false);
  await app.press("id:pane-find");
  await app.waitFor("id:find-in-terminal");
});

test("settings changes the scrollback limit and lists the shortcuts", async () => {
  const app = await open("../App.tsx", options);
  await app.press("id:settings");
  await app.press("id:settings-terminal");
  expect(app.texts().some((text) => text.includes("10,000"))).toBe(true);
  await app.press("id:scrollback-less");
  await app.waitUntil(() => app.texts().some((text) => text.includes("9,000")));
  await app.press("id:scrollback-more");
  await app.waitUntil(() => app.texts().some((text) => text.includes("10,000")));
  await app.press("id:settings-shortcuts");
  expect(app.texts()).toContain("Close terminal");
  expect(app.texts()).toContain("Previous terminal");
  expect(app.texts()).toContain("Find in terminal");
});
