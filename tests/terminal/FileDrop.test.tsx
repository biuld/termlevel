import { expect, open, test } from "nsh/test";

test("file drops paste literal Unicode paths into the target pane without submitting", async () => {
  const app = await open("./fixtures/FileDropFixture.tsx", { platform: "macos", width: 1200, height: 700 });
  try {
    const target = "id:drop-terminal-1";
    await app.type(target, "printf '\\nDROP_ARG=<%s>\\n' ");
    // Focus the first pane again: the drop must select the second pane.
    await app.press("id:drop-terminal-0");
    await app.dropFiles(target, ["file:///tmp/%E6%88%AA%E5%9B%BE%20a.png", "file:///tmp/a%27b%24%28echo%20oops%29.jpg"]);
    await app.wait(100);
    expect(app.texts()).toContain("ACTIVE_1");
    expect(app.texts().some(t => t.includes("DROP_ARG=</tmp/"))).toBe(false);
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(t => t.includes("DROP_ARG=</tmp/截图 a.png>")));
    expect(app.texts().some(t => t.includes("DROP_ARG=</tmp/a'b$(echo oops).jpg>"))).toBe(true);
    // The other session never received the drop or the command.
    await app.type("id:drop-terminal-0", "printf '\\nOTHER_PANE_OK\\n'");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().includes("OTHER_PANE_OK"));
  } finally { app.dispose(); }
});

test("modal input blocking and nonlocal image drops do not write terminal input", async () => {
  const app = await open("./fixtures/FileDropFixture.tsx", { platform: "macos", width: 1000, height: 700 });
  try {
    const target = "id:drop-terminal-0";
    await app.type(target, "printf '\\nDROP_UNTOUCHED=<%s>\\n' ");
    await app.press("id:block-drop");
    await app.dropFiles(target, ["file:///tmp/blocked.png"]);
    await app.press("id:block-drop");
    await app.dropFiles(target, ["https://example.com/image.png"]);
    await app.press(target);
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().includes("DROP_UNTOUCHED=<>"));
  } finally { app.dispose(); }
});
