import { expect, open, test } from "nsh/test";

test("native files preview image, video, audio, markdown, and an unsupported document", async () => {
  const app = await open("./fixtures/NativePreviewFixture.tsx", { platform: "macos", width: 1100, height: 720 });
  try {
    await app.press("id:file-image-png");
    await app.waitFor("label:Image preview loaded");
    await app.screenshot("artifacts/files-image-preview-native.png");
    await app.press("id:file-video-mp4");
    await app.waitFor("label:Media preview loaded");
    await app.screenshot("artifacts/files-video-preview-native.png");
    await app.press("id:file-audio-wav");
    await app.waitFor("label:Media preview loaded");
    await app.screenshot("artifacts/files-audio-preview-native.png");
    await app.press("id:file-notes-md");
    await app.waitFor("id:files-editor");
    await app.press("id:files-preview-toggle");
    await app.waitFor("id:file-text-preview");
    await app.screenshot("artifacts/files-markdown-preview-native.png");
    await app.press("id:file-drawing-svg");
    await app.waitFor("label:Image preview loaded");
    await app.press("id:files-preview-toggle");
    await app.waitFor("id:files-editor");
    await app.press("id:file-invalid-mp4");
    await app.waitFor("text:This file could not be previewed.");
    await app.press("id:file-report-pdf");
    await app.waitFor("id:files-open-with");
    expect(app.shows("text:This file is binary. Open it with the default app.")).toBe(true);
  } finally { app.dispose(); }
});
