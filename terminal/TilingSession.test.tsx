import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1400, height: 900, colorScheme: "dark" as const };
test("two real PTYs render concurrently and route input to the clicked pane while layout edits preserve processes", async () => {
  const app = await open("./fixtures/TilingFixture.tsx", options);
  async function sessions(): Promise<Array<{ id: string; pid: number; state: string }>> {
    await app.press("id:read-sessions");
    return JSON.parse(app.texts().find(t => t.startsWith('[{"id":')) ?? "[]");
  }
  async function command(id: string, text: string) {
    await app.type(`id:terminal-${id}`, text); await app.keyboard.press("enter");
  }
  try {
    const left = (await sessions())[0];
    expect(!!left).toBe(true);
    await command(left.id, "printf 'LEFT_%s\\n' START");
    await app.waitFor("text:LEFT_START");
    await app.press("id:split-right");
    const all = await sessions(), right = all.find(s => s.id !== left.id)!;
    expect(all.length).toBe(2);
    await command(right.id, "for i in 1 2 3; do printf 'RIGHT_STREAM_%s\\n' $i; sleep 0.05; done");
    await command(left.id, "for i in 1 2 3; do printf 'LEFT_STREAM_%s\\n' $i; sleep 0.05; done");
    await app.waitFor("text:RIGHT_STREAM_3"); await app.waitFor("text:LEFT_STREAM_3");
    expect(app.shows(`id:terminal-${left.id} >> text:RIGHT_STREAM_3`)).toBe(false);
    expect(app.shows(`id:terminal-${right.id} >> text:LEFT_STREAM_3`)).toBe(false);
    await command(left.id, "printf 'LEFT_COLS_'; stty size");
    await app.waitUntil(() => app.texts().some(t => /^LEFT_COLS_\d/.test(t)));
    const splitSize = app.texts().find(t => t.startsWith("LEFT_COLS_"));
    await app.keyboard.press("mod+alt+shift+right");
    await app.press("id:zoom-pane");
    await command(left.id, "printf 'ZOOM_COLS_'; stty size");
    await app.waitUntil(() => app.texts().some(t => /^ZOOM_COLS_\d/.test(t)));
    const zoomSize = app.texts().find(t => t.startsWith("ZOOM_COLS_"));
    expect(splitSize?.replace("LEFT_COLS_", "") === zoomSize?.replace("ZOOM_COLS_", "")).toBe(false);
    await app.press("id:zoom-pane");
    await app.press("id:terminal-actions"); await app.chooseMenuItem("Hide pane");
    expect((await sessions()).find(s => s.id === left.id)?.pid).toBe(left.pid);
    expect(app.shows(`id:terminal-${right.id}`)).toBe(true);
    await app.press(`id:tab-${left.id}`);
    await command(left.id, "printf 'REBIND_%s\\n' KEPT");
    await app.waitFor("text:REBIND_KEPT");
    expect(app.count(`id:terminal-${left.id}`)).toBe(1);
    expect((await sessions()).find(s => s.id === right.id)?.pid).toBe(right.pid);
    await command(left.id, "exit 3");
    await app.waitFor(`id:session-status-${left.id}`);
    expect(app.shows(`id:tab-${left.id}`)).toBe(true);
    await app.press("id:terminal-actions");
    await app.chooseMenuItem("Show terminal on the right > Terminal 2");
    await app.press(`id:terminal-${right.id}`);
    await app.keyboard.press("ctrl+d");
    await app.waitUntil(() => !app.shows(`id:tab-${right.id}`));
    expect(app.shows(`id:terminal-${left.id}`)).toBe(true);
    expect(app.shows(`id:session-status-${left.id}`)).toBe(true);
    expect((await sessions()).some(s => s.id === right.id)).toBe(false);
    await app.keyboard.press("mod+w");
    expect((await sessions()).length).toBe(0);
  } finally {
    try { await app.window("main").pressClose(); } catch (error) { if (!String(error).includes("the host left")) throw error; }
    app.dispose();
  }
});
