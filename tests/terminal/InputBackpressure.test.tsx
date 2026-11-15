import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1100, height: 750 };

async function close(app: Awaited<ReturnType<typeof open>>) {
  try { await app.window("main").pressClose(); }
  catch (error) { if (!String(error).includes("the host left")) throw error; }
  app.dispose();
}

test("a rejected paste is recorded in notifications without a terminal status bar", async () => {
  const app = await open("./fixtures/WorkspaceFixture.tsx", options);
  try {
    await app.paste("id:session-panel", { text: "x".repeat(1024 * 1024 + 1) });
    await app.waitFor("id:notifications-unread");
    expect(app.texts().some(text => text.includes("Input not sent:"))).toBe(false);
    await app.type("id:session-panel", "printf 'INPUT_RECOVERED_%s\\n' OK");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("INPUT_RECOVERED_OK")));
    await app.press("id:notifications");
    await app.waitUntil(() => app.texts().some(text => text.includes("Input not sent: paste exceeds")));
    expect(app.texts().some(text => text.includes("Terminal input unavailable"))).toBe(false);
  } finally { await close(app); }
});

test("a real client receives every reply after flooding queries before reading stdin", async () => {
  const app = await open("./fixtures/ProtocolsFixture.tsx", options);
  try {
    await app.type("id:protocol-terminal", "python3 tests/probes/input_backpressure_probe.py");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("INPUT_REPLIES_OK count=20000")));
    expect(app.texts().some(text => text.includes("Input not sent:") || text.includes("Terminal input unavailable"))).toBe(false);
  } finally { await close(app); }
});
