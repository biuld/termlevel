import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1100, height: 750 };
async function close(app: Awaited<ReturnType<typeof open>>) {
  try { await app.window("main").pressClose(); }
  catch (error) { if (!String(error).includes("the host left")) throw error; }
  app.dispose();
}

test("background terminal notifications reach the panel and navigate to their source", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.type("id:session-panel", "sleep 0.5; printf '\\033]777;notify;Build finished;All tests passed\\007'");
    await app.keyboard.press("enter");
    await app.keyboard.press("mod+t");
    await app.waitFor("id:notifications-unread");
    await app.press("id:toggle-sidebar");
    await app.press("id:notifications");
    await app.waitFor("id:notification-panel");
    expect(app.texts()).toContain("Build finished");
    expect(app.texts()).toContain("All tests passed");
    expect(app.shows("id:notifications-unread")).toBe(false);
    await app.press("label:Open source of Build finished");
    expect(app.shows("id:notification-panel")).toBe(false);
    // Returning to the source must retain its shell and output.
    await app.type("id:session-panel", "printf 'SOURCE_%s\\n' ALIVE");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("SOURCE_ALIVE")));
    await app.keyboard.press("mod+w");
    await app.press("id:notifications");
    expect(app.texts()).toContain("Build finished");
    expect(app.texts()).toContain("Terminal closed");
    await app.press("id:notifications-clear");
    await app.waitFor("id:notifications-empty");
    await app.keyboard.press("escape");
    expect(app.shows("id:notification-panel")).toBe(false);
  } finally { await close(app); }
});

test("app input warnings appear only in notifications while terminal input keeps working", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.paste("id:session-panel", { text: "x".repeat(1024 * 1024 + 1) });
    await app.waitFor("id:notifications-unread");
    expect(app.texts().some(text => text.includes("Input not sent:"))).toBe(false);
    await app.type("id:session-panel", "printf 'RECOVER_%s\\n' OK");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("RECOVER_OK")));
    await app.press("id:notifications");
    await app.waitFor("text:Input not sent");
    expect(app.texts().some(text => text.includes("App · warning"))).toBe(true);
    await app.press("label:Dismiss Input not sent notification");
    await app.waitUntil(() => !app.texts().some(text => text === "Input not sent"));
  } finally { await close(app); }
});

test("notification history scrolls after three entries and preserves both OSC forms", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.type("id:session-panel", "for n in 1 2 3 4 5; do printf '\\033]9;Notice %s\\007' $n; done");
    await app.keyboard.press("enter");
    await app.waitFor("id:notifications-unread");
    await app.setViewport(640, 420);
    await app.press("id:notifications");
    await app.waitUntil(() => app.texts().some(text => text.includes("5 / 100")));
    expect(app.texts()).toContain("Notice 5");
    expect(app.texts().some(text => text.includes("Scroll for more"))).toBe(true);
    await app.scroll("id:notifications-history", 600);
    await app.waitFor("text:Notice 1");
    await app.press("id:notifications-clear");
    await app.waitFor("id:notifications-empty");
  } finally { await close(app); }
});

test("an exited pane keeps its output and offers a new terminal without a bottom status bar", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.type("id:session-panel", "printf 'SAVED_%s\\n' OUTPUT; exit 7");
    await app.keyboard.press("enter");
    await app.waitFor("text:Exited · 7");
    expect(app.texts().some(text => text.includes("SAVED_OUTPUT"))).toBe(true);
    expect(app.texts().some(text => text.includes("Process exited with code"))).toBe(false);
    await app.press("id:notifications");
    await app.waitFor("text:Terminal exited");
    expect(app.texts()).toContain("Process exited with code 7.");
    await app.press("id:notifications-clear");
    await app.keyboard.press("escape");
    expect(app.shows("text:Exited · 7")).toBe(true);
    await app.press("label:Open new terminal");
    await app.type("label:Terminal Terminal 2", "printf 'FRESH_%s\\n' SHELL");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("FRESH_SHELL")));
  } finally { await close(app); }
});
