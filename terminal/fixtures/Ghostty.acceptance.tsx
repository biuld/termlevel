import { expect, open, test } from "nsh/test";

// A fixed canvas exposes the visible window as one semantic region.
async function waitForOutput(app: Awaited<ReturnType<typeof open>>, text: string) {
  await app.waitUntil(() => app.texts().some(value => value.includes(text)));
}

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
    await waitForOutput(app, "(END)");
    expect(app.texts().flatMap(text => text.split("\n"))).toContain("200");
    await app.screenshot(`artifacts/ghostty-production-${colorScheme}-less.png`);
    await app.keyboard.press("q");
    await app.waitUntil(() => app.texts().join("").includes("GHOSTTY_PROD_中文"));
    await app.screenshot(`artifacts/ghostty-production-${colorScheme}.png`);
    // ED2 must retain history in the emulator while excluding it from the
    // pinned visible grid, including this non-integral pane height.
    await app.type("id:session-panel", "printf '\\033[H\\033[2JGRID_CLEAR_%s\\n' OK");
    await app.keyboard.press("enter");
    await waitForOutput(app, "GRID_CLEAR_OK");
    expect(app.texts().join("\n").includes("GHOSTTY_PROD_")).toBe(false);
    await app.screenshot(`artifacts/ghostty-production-${colorScheme}-clear.png`);
  });
}
