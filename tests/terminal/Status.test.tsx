import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1000, height: 700 };

async function close(app: Awaited<ReturnType<typeof open>>) {
  try { await app.window("main").pressClose(); }
  catch (error) { if (!String(error).includes("the host left")) throw error; }
  app.dispose();
}

async function emit(app: Awaited<ReturnType<typeof open>>, sequence: string) {
  await app.type("id:session-panel", sequence);
  await app.keyboard.press("enter");
}

test("OSC 9;4 draws the title-column progress line and remove clears it", async () => {
  const app = await open("./fixtures/StatusFixture.tsx", options);
  try {
    await emit(app, "printf '\\033]9;4;1;42\\007'");
    await app.waitFor("label:Terminal 1, Terminal progress 42%");
    await emit(app, "printf '\\033]9;4;3\\007'");
    await app.waitFor("label:Terminal 1, Terminal progress unknown");
    await emit(app, "printf '\\033]9;4;0\\007'");
    await app.waitUntil(() =>
      !app.shows("label:Terminal 1, Terminal progress unknown")
      && !app.shows("label:Terminal 1, Terminal progress 42%"));
  } finally { await close(app); }
});

test("OSC 7501 uses resolved progress and keeps blocked after viewing", async () => {
  const app = await open("./fixtures/StatusFixture.tsx", options);
  try {
    await emit(app, "printf '\\033]9;4;1;42\\007'");
    await emit(app, "printf '\\033]7501;state=working:app=cargo:progress=40\\007'");
    await app.waitFor("label:Terminal 1, Working, Terminal progress 42%, cargo");
    // The line and tab label use the same resolved 9;4 percentage.
    await app.waitFor("label:Terminal progress 42%");
    expect(app.shows("label:Terminal progress 40%")).toBe(false);
    await emit(app, "printf '\\033]7501;state=done:app=cargo\\007'");
    await app.waitFor("label:Terminal 1, Done, Terminal progress 42%, cargo");
    await emit(app, "printf '\\033]7501;state=blocked:kind=permission:app=terraform:msg=QXBwbHk/\\007'");
    await app.waitFor("label:Terminal 1, Needs approval, Terminal progress 42%, terraform · Apply?");
    await app.waitFor("id:notifications-unread");
    await app.press("label:Terminal 1, Needs approval, Terminal progress 42%, terraform · Apply?");
    await app.waitFor("label:activity:blocked:read");
    expect(app.shows("label:Terminal 1, Needs approval, Terminal progress 42%, terraform · Apply?")).toBe(true);
    expect(app.shows("id:notifications-unread")).toBe(true);
  } finally { await close(app); }
});

test("OSC 7501 working without a percentage does not invent a progress line", async () => {
  const app = await open("./fixtures/StatusFixture.tsx", options);
  try {
    await emit(app, "printf '\\033]7501;state=working:app=cargo\\007'");
    await app.waitFor("label:Terminal 1, Working, cargo");
    expect(app.shows("label:Terminal progress unknown")).toBe(false);
    await emit(app, "printf '\\033]7501;state=blocked:kind=permission:app=cargo\\007'");
    await app.waitFor("label:Terminal 1, Needs approval, cargo");
    expect(app.shows("label:Terminal progress 100%")).toBe(false);
    expect(app.shows("label:Terminal progress unknown")).toBe(false);
  } finally { await close(app); }
});

test("a program error is acknowledged without suppressing future work", async () => {
  const app = await open("./fixtures/StatusFixture.tsx", options);
  try {
    await emit(app, "printf '\\033]7501;state=error:app=cargo:msg=QnVpbGQgZmFpbGVk\\007'");
    await app.waitFor("label:Terminal 1, Task failed, cargo · Build failed");
    await app.waitFor("id:notifications-unread");
    await app.press("label:Terminal 1, Task failed, cargo · Build failed");
    await app.waitFor("label:Terminal 1");
    await emit(app, "printf '\\033]7501;state=error:app=cargo:msg=QnVpbGQgZmFpbGVk\\007'");
    await app.waitFor("label:activity:none:read");
    await emit(app, "printf '\\033]7501;state=working:app=cargo\\007'");
    await app.waitFor("label:Terminal 1, Working, cargo");
  } finally { await close(app); }
});

test("unknown pause and error never report a fictional percentage", async () => {
  const app = await open("./fixtures/StatusFixture.tsx", options);
  try {
    await emit(app, "printf '\\033]9;4;4\\007'");
    await app.waitFor("label:Terminal 1, Terminal progress paused · unknown");
    expect(app.shows("label:Terminal progress 100%")).toBe(false);
    await emit(app, "printf '\\033]9;4;2\\007'");
    await app.waitFor("label:Terminal 1, Terminal progress error · unknown");
    await emit(app, "printf '\\033]9;4;1;100\\007\\033]7501;state=working:app=cargo\\007'");
    await app.waitFor("label:Terminal 1, Working, Terminal progress 100%, cargo");
    await emit(app, "printf '\\033]133;A\\007'");
    await app.waitFor("label:Terminal 1");
  } finally { await close(app); }
});

test("remounting an active native input preserves unseen program results", async () => {
  const app = await open("./fixtures/StatusFocusFixture.tsx", options);
  try {
    await app.waitFor("label:passive-focus:1");
    // This fixture mounts the production surface directly, without pane chrome.
    await app.type("label:Terminal Terminal 1", "printf '\\033]7501;state=done:app=cargo\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:activity:done:unread");
    await app.press("id:toggle-attachment");
    await app.press("id:toggle-attachment");
    await app.waitFor("label:passive-focus:2");
    expect(app.shows("label:activity:done:unread")).toBe(true);
  } finally {
    try { await app.press("id:close-status-session"); } finally { app.dispose(); }
  }
});
