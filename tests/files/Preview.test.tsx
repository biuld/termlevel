import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1100, height: 720 };

test("Markdown switches between the editor and the rendered unsaved text", async () => {
  const app = await open("./fixtures/FilesFixture.tsx", options);
  try {
    await app.press("id:file-readme-md");
    await app.waitFor("id:files-editor");
    await app.type("id:files-editor", "updated ");
    await app.press("id:files-preview-toggle");
    await app.waitFor("id:file-text-preview");
    expect(app.texts()).toContain("updated hello");
    await app.press("id:files-preview-toggle");
    await app.waitFor("id:files-editor");
    await app.press("id:file-notes-txt");
    await app.waitFor("text:Discard unsaved changes?");
  } finally { app.dispose(); }
});

test("image and media choose their preview and unsupported files open externally", async () => {
  const app = await open("./fixtures/FilesFixture.tsx", options);
  try {
    await app.press("id:file-photo-png");
    await app.waitFor("id:file-image-preview");
    await app.press("id:file-movie-mp4");
    await app.waitFor("id:file-media-preview");
    await app.press("id:file-song-mp3");
    await app.waitFor("id:file-media-preview");
    await app.press("id:file-report-pdf");
    await app.waitFor("id:files-open-with");
    await app.press("id:files-open-with");
    await app.waitFor("text:open:report.pdf");
  } finally { app.dispose(); }
});


test("HTML and SVG keep editable source alongside their previews", async () => {
  const app = await open("./fixtures/FilesFixture.tsx", options);
  try {
    await app.press("id:file-page-html");
    await app.waitFor("id:files-editor");
    await app.press("id:files-preview-toggle");
    await app.waitFor("text:Page title");
    await app.press("id:file-drawing-svg");
    await app.waitFor("id:file-image-preview");
    await app.press("id:files-preview-toggle");
    await app.waitFor("id:files-editor");
    expect(app.texts()).toContain("<svg></svg>");
  } finally { app.dispose(); }
});

test("the save action appears only while the open file has unsaved edits", async () => {
  const app = await open("./fixtures/FilesFixture.tsx", options);
  try {
    await app.press("id:file-readme-md");
    await app.waitFor("id:files-editor");
    expect(app.shows("id:files-save")).toBe(false);
    await app.type("id:files-editor", "changed ");
    await app.waitFor("id:files-save");
    await app.press("id:files-preview-toggle");
    expect(app.shows("id:files-save")).toBe(true);
    await app.press("id:files-save");
    await app.waitUntil(() => !app.shows("id:files-save"));
    expect(app.texts()).toContain("save:readme.md:changed hello");
  } finally { app.dispose(); }
});
