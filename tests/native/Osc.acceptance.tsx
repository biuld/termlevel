import { open, test } from "nsh/test";

// Native toolbar/prompt mapping. OSC 22's actual NSCursor is verified by hand:
// headless and screenshots cannot read the pointer shape.
test("native OSC 7 live cwd and OSC 133 prompt lifecycle", async () => {
  const app = await open("./tests/terminal/fixtures/OscFixture.tsx", { platform: "macos", width: 900, height: 600 });
  try {
    await app.type("id:session-panel", "printf '\\033]7;file://localhost/private/tmp\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Current directory /private/tmp");
    await app.screenshot("artifacts/native-osc-live-cwd.png");

    await app.type("id:session-panel", "printf '\\033]7501;state=working:app=cargo\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1, Working, cargo");
    await app.type("id:session-panel", "printf '\\033]133;A\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("label:Terminal 1");
    await app.screenshot("artifacts/native-osc-prompt.png");
  } finally { app.dispose(); }
});
