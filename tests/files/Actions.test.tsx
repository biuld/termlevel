import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1100, height: 720 };

async function editReadme() {
  const app = await open("./fixtures/FilesFixture.tsx", options);
  await app.press("id:file-readme-md");
  await app.waitFor("text:hello");
  await app.type("id:files-editor", "!");
  await app.waitFor("text:!hello");
  return app;
}

test("a row context menu acts on that row without discarding the open edit", async () => {
  const app = await editReadme();
  expect(app.shows("id:files-actions")).toBe(false);
  await app.rightClick("id:file-notes-txt");
  await app.chooseMenuItem("Reveal in Finder");
  expect(app.texts()).toContain("reveal:notes.txt");
  expect(app.shows("text:Discard unsaved changes?")).toBe(false);
  expect(app.shows("text:!hello")).toBe(true);
});

test("closing Files or returning to the terminal asks before discarding an edit", async () => {
  const app = await editReadme();
  await app.press("id:close-files");
  await app.waitFor("text:Discard unsaved changes?");
  await app.press("text:Keep editing");
  expect(app.shows("id:files-panel")).toBe(true);
  await app.press("id:show-terminal");
  await app.waitFor("text:Discard unsaved changes?");
  await app.press("id:files-save-leave");
  await app.waitFor("text:save:readme.md:!hello");
  expect(app.shows("id:files-panel")).toBe(false);
});

test("discarding an edit closes Files", async () => {
  const app = await editReadme();
  await app.press("id:close-files");
  await app.waitFor("text:Discard unsaved changes?");
  await app.press("id:files-discard");
  expect(app.shows("id:files-panel")).toBe(false);
  expect(app.shows("text:save:readme.md:!hello")).toBe(false);
});
