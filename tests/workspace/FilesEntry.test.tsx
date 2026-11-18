import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1200, height: 780, colorScheme: "dark" as const, modules: { "termlevel/native": { "Runtime.previewMode": true } } };

test("the title bar and group options open Files for the current group", async () => {
  const app = await open("./fixtures/PreviewApp.tsx", options);
  await app.press("id:titlebar-files");
  await app.waitFor("id:files-view");
  await app.waitFor("id:files-tab-termlevel");
  expect(app.texts()).toContain("Files · Termlevel");
  expect(app.texts()).toContain("Files are unavailable in preview.");
  await app.press("id:close-files-termlevel");
  expect(app.shows("id:files-tab-termlevel")).toBe(false);
  await app.press("id:group-menu-termlevel");
  await app.press("id:group-editor-files");
  await app.waitFor("id:files-view");
  await app.press("id:workspace-ungrouped");
  await app.press("id:titlebar-files");
  expect(app.shows("id:files-view")).toBe(false);
  app.dispose();
});
