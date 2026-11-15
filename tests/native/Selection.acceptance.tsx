import { expect, open, test } from "nsh/test";
const fill = "i=0; while [ $i -lt 800 ]; do printf 'SELECT_%04d abc 中文\\r\\n' $i; i=$((i+1)); done; printf 'SELECTION_READY\\r\\n'";
const selected = (app: Awaited<ReturnType<typeof open>>) => JSON.parse(app.texts().find(text => text.startsWith('{"cols":')) ?? "{}");
const top = (app: Awaited<ReturnType<typeof open>>) => Math.min(...app.texts().flatMap(text => text.split("\n")).filter(text => /^SELECT_\d/.test(text)).map(text => Number(text.match(/^SELECT_(\d+)/)![1])));

test("native captured selection auto-scrolls history and thumb navigation preserves selection", async () => {
  const app = await open("./tests/terminal/fixtures/SelectionFixture.tsx", { platform: "macos", width: 900, height: 600 });
  try {
    await app.type("id:selection-terminal", fill);
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("SELECTION_READY")));
    const before = top(app);
    const id = app.texts().find(text => text.startsWith("SELECTION_SESSION_"))!.slice("SELECTION_SESSION_".length);
    await app.drag(`id:terminal-grid-${id}`, { y: -900 });
    await app.waitUntil(() => selected(app).lines > 30 && top(app) < before - 20);
    await app.wait(150);
    const selection = JSON.stringify(selected(app)), origin = top(app);
    await app.wait(250);
    expect(JSON.stringify(selected(app))).toBe(selection);
    expect(top(app)).toBe(origin);
    await app.screenshot("artifacts/native-selection-autoscroll.png");
    await app.drag("label:Terminal history scrollbar", { y: -700 });
    await app.waitUntil(() => top(app) < 5);
    expect(JSON.stringify(selected(app))).toBe(selection);
    await app.screenshot("artifacts/native-selection-history-top.png");
    await app.drag("label:Terminal history scrollbar", { y: 700 });
    await app.waitUntil(() => app.texts().some(text => text.includes("SELECTION_READY")));
    expect(JSON.stringify(selected(app))).toBe(selection);
    await app.screenshot("artifacts/native-selection-history-bottom.png");
  } finally { app.dispose(); }
});

test("native downward edge selection advances from the top of history", async () => {
  const app = await open("./tests/terminal/fixtures/SelectionFixture.tsx", { platform: "macos", width: 900, height: 600 });
  try {
    await app.type("id:selection-terminal", fill);
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("SELECTION_READY")));
    await app.drag("label:Terminal history scrollbar", { y: -700 });
    await app.waitUntil(() => top(app) < 5);
    const id = app.texts().find(text => text.startsWith("SELECTION_SESSION_"))!.slice("SELECTION_SESSION_".length);
    await app.drag(`id:terminal-grid-${id}`, { y: 900 });
    await app.waitUntil(() => selected(app).lines > 30 && top(app) > 20);
    await app.screenshot("artifacts/native-selection-autoscroll-down.png");
  } finally { app.dispose(); }
});


test("native overlay scrollbar hovers and fades without resizing terminal columns", async () => {
  const app = await open("./tests/terminal/fixtures/SelectionFixture.tsx", { platform: "macos", width: 900, height: 600 });
  try {
    await app.waitUntil(() => selected(app).cols > 1);
    const columns = selected(app).cols;
    await app.type("id:selection-terminal", fill);
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("SELECTION_READY")));
    expect(selected(app).cols).toBe(columns);
    await app.hover("label:Terminal history scrollbar");
    await app.wait(100);
    expect(selected(app).cols).toBe(columns);
    await app.screenshot("artifacts/native-scrollbar-overlay-hover.png");
    const id = app.texts().find(text => text.startsWith("SELECTION_SESSION_"))!.slice("SELECTION_SESSION_".length);
    await app.hover(`id:terminal-grid-${id}`);
    await app.wait(1200);
    expect(selected(app).cols).toBe(columns);
    await app.screenshot("artifacts/native-scrollbar-overlay-idle.png");
  } finally { app.dispose(); }
});
