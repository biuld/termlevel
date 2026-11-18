import { open, test } from "nsh/test";

// The shell is a temporary wrapper with no user startup files and a neutral
// prompt. Programs report OSC 7501 themselves; the agents are not launched,
// so no account, credential or private session is involved.
test("capture agent icons, status and notifications from a controlled shell", async () => {
  const app = await open("./tests/screenshots/NotificationApp.tsx", {
    platform: "macos", width: 1100, height: 760, colorScheme: "dark",
  });
  const ids = () => (app.texts().find((text) => text.startsWith("ids:"))?.slice(4).split(",").filter(Boolean) ?? []);
  try {
    await app.waitFor("id:session-panel");
    await app.waitFor("text:❯");
    await app.waitUntil(() => ids().length >= 1);
    const first = ids()[0];

    await app.type(`id:terminal-${first}`, "printf '\\033]0;Claude\\007\\033]7501;state=idle:app=claude-code\\007'; clear");
    await app.keyboard.press("enter");
    await app.waitFor("label:Claude, Idle, Claude Code");

    await app.press("id:titlebar-new-terminal");
    await app.waitUntil(() => ids().length >= 2);
    const second = ids().find((id) => id !== first)!;
    await app.waitFor(`id:terminal-${second}`);
    await app.type(`id:terminal-${second}`, "printf '\\033]0;Codex\\007\\033]7501;state=idle:app=codex\\007'; clear");
    await app.keyboard.press("enter");
    await app.waitFor("label:Codex, Idle, Codex");

    await app.press("id:titlebar-new-terminal");
    await app.waitUntil(() => ids().length >= 3);
    const third = ids().find((id) => id !== first && id !== second)!;
    await app.waitFor(`id:terminal-${third}`);
    await app.type(`id:terminal-${third}`, "printf '\\033]0;Pi\\007\\033]7501;state=idle:app=pi\\007'; clear");
    await app.keyboard.press("enter");
    await app.waitFor("label:Pi, Idle, Pi");

    await app.press(`id:tab-${first}`);
    await app.wait(400);
    await app.screenshot("docs/assets/screenshots/13-agent-status.png");

    await app.type(`id:terminal-${first}`, "printf '\\033]9;4;1;42\\007\\033]7501;state=working:app=cargo\\007'; clear");
    await app.keyboard.press("enter");
    await app.waitFor("label:Claude, Working, Terminal progress 42%, cargo");
    await app.screenshot("docs/assets/screenshots/14-status-progress.png");

    const program = "\\033]7501;id=approval:state=blocked:kind=permission:app=agent:msg=TWF5IEkgYXBwbHkgdGhlc2UgY2hhbmdlcyB0byBzcmMvbWFpbi5ycz8=\\007\\033]7501;id=build:state=error:app=cargo:msg=QnVpbGQgZmFpbGVkOiB1bnJlc29sdmVkIGltcG9ydCBpbiBzcmMvbWFpbi5ycy4=\\007";
    await app.type(`id:terminal-${first}`, `printf '\\033]777;notify;Build finished;All 128 tests passed. The build is ready.\\007${program}'; clear`);
    await app.keyboard.press("enter");
    await app.waitFor("id:notifications-unread");
    await app.press("id:notifications");
    await app.waitFor("id:notifications-history");
    await app.wait(400);
    await app.screenshot("docs/assets/screenshots/15-notifications.png");
  } finally {
    try { await app.window("main").pressClose(); }
    catch (error) { if (!String(error).includes("the host left")) throw error; }
    app.dispose();
  }
});
