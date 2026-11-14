import { expect, open, test } from "nsh/test";

// Production entry, default user shell, real Ghostty session; no preview or
// module replacement. Kept outside the default headless test discovery.
for (const colorScheme of ["dark", "light"] as const) {
  test(`production Ghostty shell, find and less (${colorScheme})`, async () => {
    const app = await open("./App.tsx", { platform: "macos", width: 1200, height: 780, colorScheme });
    await app.wait(300);
    await app.type("id:session-panel", "printf 'GHOSTTY_PROD_%s\\n' 中文");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().join("").includes("GHOSTTY_PROD_中文"));
    await app.press("id:pane-find");
    await app.type("id:find-query", "GHOSTTY_PROD_");
    await app.waitUntil(() => app.texts().some(text => /^\d+ of \d+$/.test(text)));
    await app.screenshot(`artifacts/ghostty-production-${colorScheme}-find.png`);
    await app.press("id:find-next");
    await app.press("id:find-close");
    await app.type("id:session-panel", "seq 1 200 | /usr/bin/less -R");
    await app.keyboard.press("enter");
    await app.wait(200);
    await app.keyboard.press("shift+g");
    await app.waitFor("text:(END)");
    expect(app.texts()).toContain("200");
    await app.screenshot(`artifacts/ghostty-production-${colorScheme}-less.png`);
    await app.keyboard.press("q");
    await app.waitUntil(() => app.texts().join("").includes("GHOSTTY_PROD_中文"));
    await app.screenshot(`artifacts/ghostty-production-${colorScheme}.png`);
  });
}
