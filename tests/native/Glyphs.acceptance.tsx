import { expect, open, test } from "nsh/test";

// Real /bin/sh PTY, production terminal renderer, bundled JetBrains Mono + Sarasa Term SC, 13pt.
test("bundled fonts render circled digits, CJK and coding ligatures", async () => {
  const app = await open("./tests/terminal/fixtures/StatusFixture.tsx", { platform: "macos", width: 1000, height: 700, colorScheme: "dark" });
  try {
    await app.type("id:session-panel", "printf '\\033[H\\033[2J①②③ ABC\\r\\nA①B②C③D\\r\\n中①文②字③\\r\\n①中②A③\\r\\n'; cols=$(stty size); cols=${cols##* }; printf '\\033[6;%sH①\\033[7;%sH③' \"$cols\" \"$cols\"; printf '\\033[5;1HA->B  C!=D  E=>F  == === <= >=\\033[8;1H中文 -> ABC != ①②③\\033[10;1H\\033[1mBold -> != =>\\033[0m\\033[11;1H\\033[3mItalic -> != =>\\033[0m\\033[13;1HGLYPH_READY\\r\\n'");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("GLYPH_READY")));
    expect(app.texts().join("\n").includes("A①B②C③D")).toBe(true);
    expect(app.texts().join("\n").includes("A->B  C!=D  E=>F")).toBe(true);
    await app.screenshot("artifacts/glyphs-native.png");
  } finally {
    try { await app.window("main").pressClose(); } catch {}
    app.dispose();
  }
});
