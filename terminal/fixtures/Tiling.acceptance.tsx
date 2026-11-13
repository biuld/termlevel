import { expect, open, test } from "nsh/test";

test("native tiled terminals keep geometry, IME ownership and state across pane operations", async () => {
  const app = await open("./terminal/fixtures/TilingFixture.tsx", { platform: "macos", width: 1400, height: 900, colorScheme: "dark" });
  async function sessions(): Promise<Array<{ id: string; pid: number }>> {
    await app.press("id:read-sessions");
    return JSON.parse(app.texts().find(t => t.startsWith('[{"id":')) ?? "[]");
  }
  async function probe(action: string) {
    await app.press(`id:tile-probe-${action}`); await app.wait(80);
    expect(app.texts().some(t => t.startsWith("ERROR"))).toBe(false);
    return JSON.parse(app.texts().find(t => t.startsWith('{"marked":')) ?? "null");
  }
  async function geometry() {
    await app.press("id:tile-probe-geometry"); await app.wait(80);
    expect(app.texts().some(t => t.startsWith("ERROR"))).toBe(false);
    const items: Array<{ id: string; inputX: number; inputY: number; cursorX: number; cursorY: number }> = JSON.parse(app.texts().find(t => t.startsWith('[{"id":') && t.includes('"inputX"')) ?? "[]");
    expect(items.length).toBe(2);
    for (const item of items) {
      expect(Math.abs(item.inputX - item.cursorX) < 0.5).toBe(true);
      expect(Math.abs(item.inputY - item.cursorY) < 0.5).toBe(true);
    }
  }
  try {
    const left = (await sessions())[0];
    await app.type(`id:terminal-${left.id}`, "printf 'LEFT_%s\\n' ACTIVE"); await app.keyboard.press("enter");
    await app.waitFor("text:LEFT_ACTIVE");
    await app.press("id:split-right");
    const right = (await sessions()).find(s => s.id !== left.id)!;
    await app.type(`id:terminal-${right.id}`, "printf 'RIGHT_%s\\n' ACTIVE"); await app.keyboard.press("enter");
    await app.waitFor("text:RIGHT_ACTIVE");
    await geometry();
    await app.screenshot("artifacts/tiling-native-two-panes.png");
    await app.keyboard.press("mod+alt+left");
    expect(app.shows(`id:pane-${left.id} >> id:session-toolbar`)).toBe(true);
    await app.type(`id:terminal-${left.id}`, "printf 'IME_LEFT_%s\\n' ");
    const before = await probe("read");
    const marked = await probe("mark"); expect(marked.marked).toBe(true);
    // AppKit reserves all keys for the input method while marked text exists.
    // Exercise the same layout command via the fixture without ending composition.
    await app.press("id:tile-probe-swap");
    const moved = await probe("read");
    expect(moved.marked).toBe(true);
    await app.screenshot("artifacts/tiling-native-after-swap.png");
    if (Math.abs(moved.x - before.x) <= 100) throw new Error(JSON.stringify({ before, marked, moved }));
    await probe("convert");
    await app.screenshot("artifacts/tiling-native-ime-swap.png");
    await probe("commit"); await app.keyboard.press("enter");
    await app.waitFor("text:IME_LEFT_");
    expect(app.texts().join("")).toContain("IME_LEFT_中文");
    await app.press("id:tile-probe-output");
    const outputs: Array<{ id: string; output: string }> = JSON.parse(app.texts().find(t => t.startsWith('[{"id":') && t.includes('"output"')) ?? "[]");
    expect(outputs.find(s => s.id === right.id)?.output.includes("中文")).toBe(false);
    await geometry();
    await app.drag("label:Resize terminal panes", { x: 55 });
    await geometry();
    await app.press("id:zoom-pane");
    await app.screenshot("artifacts/tiling-native-zoom.png");
    await app.press("id:zoom-pane"); await geometry();
    await app.keyboard.press("mod+shift+d");
    await app.wait(100);
    await app.screenshot("artifacts/tiling-native-three-panes.png");
    await app.setViewport(640, 480);
    await app.screenshot("artifacts/tiling-native-narrow.png");
    await app.setViewport(1400, 900);
    await app.press("id:toggle-sidebar");
    await app.hover("id:sidebar-panel");
    await app.screenshot("artifacts/tiling-native-sidebar-overlay.png");
    expect((await sessions()).find(s => s.id === left.id)?.pid).toBe(left.pid);
  } finally {
    try { await app.window("main").pressClose(); } catch (error) { if (!String(error).includes("the host left")) throw error; }
    app.dispose();
  }
});
