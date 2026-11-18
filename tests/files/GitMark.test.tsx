import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1100, height: 720 };

test("a listed root directory and file pick up Git marks when status arrives later", async () => {
  const app = await open("./fixtures/GitMarkFixture.tsx", options);
  await app.waitFor("id:file-src");
  await app.waitFor("id:file-readme-md");
  expect(app.shows("id:file-folder-src")).toBe(false);
  expect(app.shows("id:file-status-readme-md")).toBe(false);
  await app.press("id:release-git");
  await app.waitFor("id:file-folder-src");
  await app.waitFor("id:file-status-readme-md");
  expect(app.texts()).toContain("M");
  app.dispose();
});
