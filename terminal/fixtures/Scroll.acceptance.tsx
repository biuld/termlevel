import { expect, open, test } from "nsh/test";
test("native mouse-report TUI scroll profile", async () => {
  const app = await open("./terminal/fixtures/ScrollFixture.tsx", { platform: "macos", width:1200, height:780 });
  try {
    console.log(app.texts().find(text => text.startsWith("SCROLL_HOST_PID_")));
    await app.type("id:scroll-terminal", "python3 -u terminal/fixtures/scroll_probe.py");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("SCROLL_FRAME_000000")));
    await app.screenshot("artifacts/native-scroll-start.png");
    await app.press("id:scroll-start");
    await app.wait(25000);
    await app.press("id:scroll-stop");
    await app.waitUntil(() => app.texts().some(text =>
      text.startsWith("NATIVE_SURFACE_FRAMES_") && Number(text.slice("NATIVE_SURFACE_FRAMES_".length)) > 10));
    expect(app.texts().some(text => /SCROLL_FRAME_000000/.test(text))).toBe(false);
    await app.screenshot("artifacts/native-scroll-end.png");
    await app.keyboard.press("q");
    await app.waitUntil(() => app.texts().some(text => text.includes("SCROLL_PROBE_DONE")));
  } finally { app.dispose(); }
}, 60000);

test("native scroll publishes completed frames between consecutive sync transactions", async () => {
  const app = await open("./terminal/fixtures/ScrollFixture.tsx", { platform: "macos", width:1200, height:780 });
  try {
    await app.type("id:scroll-terminal", "python3 -u terminal/fixtures/scroll_probe.py --overlap-sync");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("SCROLL_FRAME_000000")));
    await app.press("id:scroll-start");
    // Require repeated progress while wheels are still arriving, rather than
    // accepting a single final redraw after stopping input or the hold timeout.
    const frame = () => app.texts().join("\n").match(/SCROLL_FRAME_(\d+)/)?.[1];
    for (let round = 0; round < 6; round++) {
      const before = frame();
      await app.wait(500);
      expect(frame() !== undefined && frame() !== before).toBe(true);
    }
    await app.press("id:scroll-stop");
    await app.screenshot("artifacts/native-scroll-consecutive-sync.png");
    await app.keyboard.press("q");
    await app.waitUntil(() => app.texts().some(text => text.includes("SCROLL_PROBE_DONE")));
  } finally { app.dispose(); }
}, 30000);
