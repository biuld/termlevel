import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1000, height: 700 };

async function closeWindow(app: Awaited<ReturnType<typeof open>>) {
  try {
    await app.window("main").pressClose();
  } catch (error) {
    // Native macOS ends its host when the last window closes. The driver can
    // observe the disconnect before the close request's reply arrives.
    if (!String(error).includes("the host left")) throw error;
  }
  app.dispose();
}

test("a real shell survives native view detach and rebind and reports its exit", async () => {
  const app = await open("./fixtures/SessionFixture.tsx", options);
  try {
    await app.type("id:live-terminal", "printf 'NATIVE_%s\\n' PTY");
    await app.keyboard.press("enter");
    await app.waitFor("text:NATIVE_PTY");
    await app.paste("id:live-terminal", { text: "printf 'PASTE_%s\\n' 中文" });
    await app.keyboard.press("enter");
    await app.waitFor("text:PASTE_");
    expect(app.texts().join("")).toContain("PASTE_中文");
    await app.setViewport(620, 300);
    await app.press("id:detach-live");
    expect(app.shows("id:live-terminal")).toBe(false);
    await app.press("id:detach-live");
    await app.waitFor("text:NATIVE_PTY");
    await app.type("id:live-terminal", "printf 'REBOUND_%s\\n' SESSION; exit 9");
    await app.keyboard.press("enter");
    await app.waitFor("text:REBOUND_SESSION");
    await app.waitFor("text:exited");
    expect(app.texts()).toContain("9");
    await app.press("id:detach-live");
    await app.press("id:detach-live");
    await app.waitFor("text:REBOUND_SESSION");
  } finally {
    await app.press("id:close-live");
    app.dispose();
  }
});

test("the production workspace uses a real PTY and keeps it across UI operations", async () => {
  const app = await open("./fixtures/WorkspaceFixture.tsx", options);
  try {
    await app.type("id:session-panel", "printf 'WORKSPACE_%s\\n' PTY");
    await app.keyboard.press("enter");
    await app.waitFor("text:WORKSPACE_PTY");
    expect(app.shows("text:Interactive UI preview")).toBe(false);
    await app.press("id:toggle-sidebar");
    await app.waitFor("text:WORKSPACE_PTY");
    await app.press("id:toggle-sidebar");
    await app.keyboard.press("mod+t");
    await app.keyboard.press("mod+w");
    await app.waitFor("text:WORKSPACE_PTY");
    await app.press("id:settings");
    await app.press("id:appearance-light");
    await app.press("label:Close settings");
    await app.waitFor("text:WORKSPACE_PTY");
  } finally {
    await closeWindow(app);
  }
});

test("recreating the whole TSX workspace rebinds Rust sessions", async () => {
  const app = await open("./fixtures/WorkspaceRebindFixture.tsx", options);
  try {
    await app.type("id:session-panel", "printf 'ROOT_%s\\n' REBOUND");
    await app.keyboard.press("enter");
    await app.waitFor("text:ROOT_REBOUND");
    await app.press("id:toggle-workspace");
    expect(app.shows("id:session-panel")).toBe(false);
    await app.press("id:toggle-workspace");
    await app.waitFor("text:ROOT_REBOUND");
    await app.type("id:session-panel", "printf 'AFTER_%s\\n' REBIND");
    await app.keyboard.press("enter");
    await app.waitFor("text:AFTER_REBIND");
  } finally {
    await closeWindow(app);
  }
});
