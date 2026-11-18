import { expect, open, test } from "nsh/test";

test("native sidebar shows idle agent icons in expanded and collapsed layouts", async () => {
  const app = await open("./tests/terminal/fixtures/StatusFixture.tsx", { platform: "macos", width: 900, height: 600 });
  try {
    await app.type("id:session-panel", "printf '\\033]7501;state=idle:app=codex\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1, Idle, Codex");
    await app.screenshot("artifacts/native-agent-codex.png");
    await app.type("id:session-panel", "printf '\\033]7501;state=idle:app=claude-code\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1, Idle, Claude Code");
    await app.screenshot("artifacts/native-agent-claude.png");
    for (const [id, name] of [["grok", "Grok"], ["pi", "Pi"], ["opencode", "OpenCode"]]) {
      await app.type("id:session-panel", `printf '\\033]7501;state=idle:app=${id}\\007'`);
      await app.keyboard.press("enter");
      await app.waitFor(`label:Terminal 1, Idle, ${name}`);
      await app.screenshot(`artifacts/native-agent-${id}.png`);
    }
    await app.press("id:toggle-sidebar");
    await app.screenshot("artifacts/native-agent-collapsed.png");
  } finally { app.dispose(); }
});

// The status/progress UI is drawn by the native sidebar. Headless tests cover
// parsing and tab labels; these captures verify the native icon and 2 pt line.
test("native sidebar status icon, progress line and collapsed rail", async () => {
  const app = await open("./tests/terminal/fixtures/StatusFixture.tsx", { platform: "macos", width: 900, height: 600 });
  try {
    await app.type("id:session-panel", "printf '\\033]9;4;1;42\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1, Terminal progress 42%");
    await app.screenshot("artifacts/native-status-progress.png");

    await app.type("id:session-panel",
      "printf '\\033]7501;state=working:app=cargo:progress=40\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1, Working, Terminal progress 42%, cargo");
    await app.screenshot("artifacts/native-status-working.png");

    await app.type("id:session-panel",
      "printf '\\033]7501;state=blocked:kind=permission:app=terraform:msg=QXBwbHk/\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1, Needs approval, Terminal progress 42%, terraform · Apply?");
    await app.screenshot("artifacts/native-status-blocked.png");

    await app.press("id:toggle-sidebar");
    await app.screenshot("artifacts/native-status-collapsed.png");
    await app.press("id:toggle-sidebar");
    await app.type("id:session-panel", "printf '\\033]9;4;0\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1, Needs approval, terraform · Apply?");
    expect(app.shows("label:Terminal progress 100%")).toBe(false);
    await app.screenshot("artifacts/native-status-blocked-no-progress.png");
    await app.hover("label:Terminal 1, Needs approval, terraform · Apply?");
    await app.waitUntil(() => app.texts().some(text => text.includes("Needs approval") && text.includes("terraform · Apply?")));
    await app.screenshot("artifacts/native-status-tooltip.png");
    await app.hover("id:session-toolbar");
    await app.press("label:Terminal 1, Needs approval, terraform · Apply?");
    // The fixture-only empty Text diagnostic is headless metadata; native
    // acceptance checks the visible product label and captures the retained icon.
    await app.waitFor("label:Terminal 1, Needs approval, terraform · Apply?");
    await app.screenshot("artifacts/native-status-blocked-read.png");
    await app.type("id:session-panel", "printf '\\033]7501;state=error:app=cargo:msg=QnVpbGQgZmFpbGVk\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1, Task failed, cargo · Build failed");
    await app.screenshot("artifacts/native-status-error.png");
    await app.type("id:session-panel", "printf '\\033]7501;state=done:app=cargo\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1, Done, cargo");
    await app.screenshot("artifacts/native-status-done.png");
    await app.press("label:Terminal 1, Done, cargo");
    await app.waitFor("label:Terminal 1");
    await app.type("id:session-panel", "printf '\\033]9;4;4\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1");
    expect(app.shows("label:Terminal progress paused · unknown")).toBe(false);
    await app.screenshot("artifacts/native-status-paused.png");
  } finally { app.dispose(); }
});
