import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1000, height: 700 };

async function close(app: Awaited<ReturnType<typeof open>>) {
  try { await app.window("main").pressClose(); }
  catch (error) { if (!String(error).includes("the host left")) throw error; }
  app.dispose();
}

async function emit(app: Awaited<ReturnType<typeof open>>, selector: string, sequence: string) {
  await app.type(selector, sequence);
  await app.keyboard.press("enter");
}

test("OSC 7 updates the toolbar, saved snapshot and new-terminal directory", async () => {
  const app = await open("./fixtures/OscFixture.tsx", options);
  try {
    await emit(app, "id:session-panel", "printf '\\033]7;file://localhost/private/tmp\\007'");
    await app.waitFor("label:Current directory /private/tmp");
    await app.wait(400);
    expect(JSON.parse(app.storage().workspace ?? "{}").tabs?.[0]?.cwd).toBe("/private/tmp");
    await app.keyboard.press("mod+t");
    await app.waitFor("label:Current directory /private/tmp");
    expect(app.texts()).toContain("Terminal 2");
  } finally { await close(app); }
});

test("OSC 133 clears working status and exposes prompt state; OSC 22 exposes shape", async () => {
  const app = await open("./fixtures/SessionFixture.tsx", options);
  try {
    await emit(app, "id:live-terminal", "printf '\\033]7;file://localhost/private/tmp\\007'");
    await app.waitFor("label:cwd:/private/tmp");
    await emit(app, "id:live-terminal", "printf '\\033]22;pointer\\007'");
    await app.waitFor("label:shape:pointer");
    await emit(app, "id:live-terminal", "printf '\\033]7501;state=working:id=build\\007'");
    await app.waitFor("label:activity:working");
    await emit(app, "id:live-terminal", "printf '\\033]133;A\\007'");
    await app.waitFor("label:prompt:yes");
    await app.waitFor("label:activity:none");
    await emit(app, "id:live-terminal", "printf '\\033]133;C\\007'");
    await app.waitFor("label:prompt:no");
  } finally { await close(app); }
});
