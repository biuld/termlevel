import { expect, open, test } from "nsh/test";

async function close(app: Awaited<ReturnType<typeof open>>) {
  try { await app.window("main").pressClose(); }
  catch (error) { if (!String(error).includes("the host left")) throw error; }
  app.dispose();
}

for (const colorScheme of ["dark", "light"] as const) {
  test(`native notification content and semantic icons in ${colorScheme}`, async () => {
    const app = await open("./tests/terminal/fixtures/NotificationFixture.tsx", {
      platform: "macos", width: 1000, height: 720, colorScheme,
    });
    try {
      const program = colorScheme === "dark"
        ? "\\033]7501;id=approval:state=blocked:kind=permission:app=agent:msg=TWF5IEkgYXBwbHkgdGhlc2UgY2hhbmdlcyB0byBzcmMvbWFpbi5ycz8=\\007\\033]7501;id=build:state=error:app=cargo:msg=QnVpbGQgZmFpbGVkOiB1bnJlc29sdmVkIGltcG9ydCBpbiBzcmMvbWFpbi5ycy4=\\007"
        : "\\033]7501;id=question:state=blocked:kind=question:app=agent:msg=V2hpY2ggZW52aXJvbm1lbnQgc2hvdWxkIEkgZGVwbG95IHRvPw==\\007\\033]7501;id=auth:state=blocked:kind=auth:app=npm:msg=U2lnbiBpbiB0byBjb250aW51ZSBwdWJsaXNoaW5nIHRoZSBwYWNrYWdlLg==\\007";
      await app.type("id:session-panel", `printf '\\033]777;notify;Build finished;All 128 tests passed. The build is ready.\\007${program}'; clear`);
      await app.keyboard.press("enter");
      await app.waitFor("id:notifications-unread");
      await app.press("id:notifications");
      await app.waitUntil(() => app.texts().some(text => text.includes("3 / 100")));
      await app.waitFor(colorScheme === "dark" ? "label:Task failed" : "label:Sign in required");
      expect(app.shows(colorScheme === "dark" ? "label:Needs approval" : "label:Needs answer")).toBe(true);
      await app.screenshot(`artifacts/native-notifications-${colorScheme}.png`);
      await app.setViewport(640, 420);
      await app.screenshot(`artifacts/native-notifications-${colorScheme}-compact.png`);
      if (colorScheme === "light") {
        await app.setViewport(360, 420);
        await app.screenshot("artifacts/native-notifications-light-narrow.png");
      }
      await app.press("id:notifications-clear");
      await app.waitFor("id:notifications-empty");
      await app.screenshot(`artifacts/native-notifications-${colorScheme}-empty.png`);
      if (colorScheme === "dark") {
        await app.keyboard.press("escape");
        const message = "Diagnostic detail. ".repeat(80) + "DETAILS_END";
        await app.type("id:session-panel", `printf '\\033]777;notify;Build details;${message}\\007'; clear`);
        await app.keyboard.press("enter");
        await app.waitFor("id:notifications-unread");
        await app.press("id:notifications");
        await app.press("label:Expand Build details notification");
        await app.waitFor(`id:notification-panel >> text:${message}`);
        await app.screenshot("artifacts/native-notifications-expanded.png");
        await app.scroll("id:notifications-history", 2000);
        await app.screenshot("artifacts/native-notifications-expanded-end.png");
        await app.press("label:Collapse Build details notification");
        await app.waitFor("label:Expand Build details notification");
      }
    } finally { await close(app); }
  });
}
