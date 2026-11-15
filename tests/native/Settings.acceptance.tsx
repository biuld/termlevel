import { expect, open, test } from "nsh/test";

test("Appearance combines theme, indicator and interface font settings", async () => {
  const app = await open("./tests/terminal/fixtures/SettingsFixture.tsx", {
    platform: "macos", width: 1200, height: 780, colorScheme: "dark",
  });
  try {
    await app.press("id:settings");
    await app.waitFor("id:appearance-dark");
    expect(app.shows("id:settings-interface")).toBe(false);
    await app.screenshot("artifacts/settings-appearance-top.png");
    await app.press("id:working-indicator-vinyl");
    await app.scroll("id:settings-content", 600);
    await app.waitFor("id:ui-font-Georgia");
    await app.screenshot("artifacts/settings-appearance-fonts.png");
    await app.press("id:ui-font-Georgia");
    await app.press("id:settings-terminal");
    await app.screenshot("artifacts/settings-terminal-after-scroll.png");
    await app.press("id:settings-appearance");
    await app.screenshot("artifacts/settings-appearance-after-switch.png");
    await app.press("label:Close settings");
  } finally {
    try { await app.window("main").pressClose(); } catch (error) { if (!String(error).includes("the host left")) throw error; }
    app.dispose();
  }
});
