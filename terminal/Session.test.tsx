import { expect, open, test } from "nsh/test";

// A fixed canvas exposes the visible window as one semantic region.
async function waitForOutput(app: Awaited<ReturnType<typeof open>>, text: string) {
  await app.waitUntil(() => app.texts().some(value => value.includes(text)));
}

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
    await waitForOutput(app, "NATIVE_PTY");
    await app.paste("id:live-terminal", { text: "printf 'PASTE_%s\\n' 中文" });
    await app.keyboard.press("enter");
    await waitForOutput(app, "PASTE_");
    expect(app.texts().join("")).toContain("PASTE_中文");
    await app.setViewport(620, 300);
    await app.press("id:detach-live");
    expect(app.shows("id:live-terminal")).toBe(false);
    await app.press("id:detach-live");
    await waitForOutput(app, "NATIVE_PTY");
    await app.type("id:live-terminal", "printf 'REBOUND_%s\\n' SESSION; exit 9");
    await app.keyboard.press("enter");
    await waitForOutput(app, "REBOUND_SESSION");
    await app.waitFor("text:exited");
    expect(app.texts()).toContain("9");
    await app.press("id:detach-live");
    await app.press("id:detach-live");
    await waitForOutput(app, "REBOUND_SESSION");
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
    await waitForOutput(app, "WORKSPACE_PTY");
    expect(app.shows("text:Interactive UI preview")).toBe(false);
    await app.press("id:toggle-sidebar");
    await waitForOutput(app, "WORKSPACE_PTY");
    await app.press("id:toggle-sidebar");
    await app.keyboard.press("mod+t");
    await app.keyboard.press("mod+w");
    await waitForOutput(app, "WORKSPACE_PTY");
    await app.press("id:settings");
    await app.press("id:appearance-light");
    await app.press("label:Close settings");
    await waitForOutput(app, "WORKSPACE_PTY");
  } finally {
    await closeWindow(app);
  }
});

test("recreating the whole TSX workspace rebinds Rust sessions", async () => {
  const app = await open("./fixtures/WorkspaceRebindFixture.tsx", options);
  try {
    await app.type("id:session-panel", "printf 'ROOT_%s\\n' REBOUND");
    await app.keyboard.press("enter");
    await waitForOutput(app, "ROOT_REBOUND");
    await app.press("id:toggle-workspace");
    expect(app.shows("id:session-panel")).toBe(false);
    await app.press("id:toggle-workspace");
    await waitForOutput(app, "ROOT_REBOUND");
    await app.type("id:session-panel", "printf 'AFTER_%s\\n' REBIND");
    await app.keyboard.press("enter");
    await waitForOutput(app, "AFTER_REBIND");
  } finally {
    await closeWindow(app);
  }
});

test("real terminal find publishes bounded Unicode results and replaces an older query", async () => {
  const app = await open("./fixtures/WorkspaceFixture.tsx", options);
  try {
    await app.type("id:session-panel", "python3 -c '[print(chr(20013)+chr(25991)+\"_\"+str(i)) for i in range(150)]'");
    await app.keyboard.press("enter");
    await waitForOutput(app, "中文_149");
    await app.press("id:pane-find");
    await app.type("id:find-query", "中文_1");
    await app.waitFor("text:1 of 61");
    await app.press("id:find-next");
    await app.waitFor("text:2 of 61");
    await app.clear("id:find-query");
    await app.type("id:find-query", "中文_149");
    await app.waitFor("text:1 of 1");
    await app.press("id:find-close");
    expect(app.shows("id:find-in-terminal")).toBe(false);
  } finally {
    await closeWindow(app);
  }
});
