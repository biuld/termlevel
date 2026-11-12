import { open, test, expect } from "nsh/test";

test("native AppKit composition commits once, cancels, and follows the grid cursor", async () => {
  const app = await open("./terminal/fixtures/InputFixture.tsx", { platform: "macos", width: 1000, height: 700 });
  const snapshot = () => JSON.parse(app.texts().find(text => text.startsWith('{"marked":')) ?? "null");
  const output = () => JSON.parse(app.texts().find(text => text.startsWith('"') && text.endsWith('"')) ?? '""');
  async function probe(action: string) {
    await app.press(`id:probe-${action}`);
    await app.wait(80);
    const errors = app.texts().filter(text => text.startsWith("ERROR"));
    expect(errors.length).toBe(0);
    return snapshot();
  }
  async function readOutput() {
    await app.press("id:probe-output-read");
    await app.wait(50);
    return output() as string;
  }
  async function checkAlignment() {
    await app.press("id:probe-geometry-read");
    await app.wait(80);
    const geometry = JSON.parse(app.texts().find(text => text.startsWith('{"inputX":')) ?? "null");
    expect(geometry !== null).toBe(true);
    console.log("input/grid geometry", geometry);
    expect(Math.abs(geometry.inputX - geometry.cursorX) < 0.5).toBe(true);
    expect(Math.abs(geometry.inputY - geometry.cursorY) < 0.5).toBe(true);
  }
  try {
    await app.wait(150);
    const initial = await probe("read");
    await app.type("id:ime-terminal", "printf 'IME_RESULT_%s\\n' ");
    const before = await probe("read");
    await checkAlignment();
    expect(before.text).toBe("");
    expect(before.x > initial.x + 50).toBe(true);
    const marked = await probe("mark");
    expect(marked.marked).toBe(true);
    expect(marked.text).toBe("zhongwen");
    const whileMarked = await readOutput();
    expect(whileMarked.includes("zhongwen")).toBe(false);
    expect(whileMarked.includes("中文")).toBe(false);
    const converted = await probe("convert");
    expect(converted.marked).toBe(true);
    expect(converted.text).toBe("中文");
    expect(Math.abs(converted.y - before.y) < 2).toBe(true);
    expect(converted.x > before.x).toBe(true);
    await app.screenshot("artifacts/native-ime-preedit.png");
    await probe("commit");
    expect((await probe("read")).text).toBe("");
    await app.keyboard.press("enter");
    await app.waitFor("text:IME_RESULT_");
    expect((await readOutput()).includes("\nIME_RESULT_中文\n")).toBe(true);
    const afterLine = await probe("read");
    expect(Math.abs(Math.abs(afterLine.y - before.y) - 44) < 3).toBe(true);
    await probe("mark");
    const cancelled = await probe("cancel");
    expect(cancelled.marked).toBe(false);
    expect((await readOutput()).includes("zhongwen")).toBe(false);
    await app.type("id:ime-terminal", "printf 'AFTER_%s\\n' CANCEL");
    await app.keyboard.press("enter");
    await app.waitFor("text:AFTER_CANCEL");
    // Leaving an AppKit field editor commits its marked text to its own
    // session. It must never migrate to the newly focused session.
    await probe("mark");
    await probe("convert");
    await app.press("id:probe-switch");
    await app.wait(100);
    await app.press("id:probe-other-read");
    await app.wait(50);
    expect((output() as string).includes("中文")).toBe(false);
    await app.type("id:other-terminal", "printf 'OTHER_%s\\n' FOCUSED");
    await app.keyboard.press("enter");
    await app.waitFor("text:OTHER_FOCUSED");
    await app.press("id:probe-switch");
    await app.wait(100);
    const original = await readOutput();
    expect(original.includes("zhongwen")).toBe(false);
    expect(original.includes("中文")).toBe(true);
    await app.keyboard.press("ctrl+u");
    await app.type("id:ime-terminal", "printf 'RESTORED_%s\\n' FOCUS");
    await app.keyboard.press("enter");
    await app.waitFor("text:RESTORED_FOCUS");
    await app.type("id:ime-terminal", "i=0; while [ $i -lt 65 ]; do printf 'SCROLL_%s\\n' $i; i=$((i+1)); done");
    await app.keyboard.press("enter");
    await app.waitFor("text:SCROLL_64");
    const scrolled = await probe("read");
    await checkAlignment();
    await probe("mark");
    const scrollComposition = await probe("convert");
    expect(Math.abs(scrollComposition.y - scrolled.y) < 2).toBe(true);
    await probe("cancel");
    await app.setViewport(640, 360);
    const resized = await probe("read");
    expect(resized.height > 0).toBe(true);
    await checkAlignment();
    await app.type("id:ime-terminal", "printf 'SIZE_BEFORE_'; stty size");
    await app.keyboard.press("enter");
    await app.wait(100);
    const beforeSize = (await readOutput()).match(/\nSIZE_BEFORE_(\d+) (\d+)/);
    expect(beforeSize !== null).toBe(true);
    await app.press("id:probe-font");
    await app.wait(100);
    await app.type("id:ime-terminal", "printf 'SIZE_AFTER_'; stty size");
    await app.keyboard.press("enter");
    await app.wait(100);
    const afterSize = (await readOutput()).match(/\nSIZE_AFTER_(\d+) (\d+)/);
    expect(afterSize !== null).toBe(true);
    expect(Number(afterSize![1]) < Number(beforeSize![1])).toBe(true);
    expect(Number(afterSize![2]) < Number(beforeSize![2])).toBe(true);
    await checkAlignment();
    await app.screenshot("artifacts/native-ime-after.png");
  } finally {
    try { await app.window("main").pressClose(); }
    catch (error) { if (!String(error).includes("the host left")) throw error; }
    app.dispose();
  }
});
