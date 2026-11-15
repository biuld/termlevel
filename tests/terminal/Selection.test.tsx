import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 900, height: 600 };
const fill = "i=0; while [ $i -lt 800 ]; do printf 'SELECT_%04d abc 中文\\r\\n' $i; i=$((i+1)); done; printf 'SELECTION_READY\\r\\n'";
const selected = (app: Awaited<ReturnType<typeof open>>) => JSON.parse(app.texts().find(text => text.startsWith('{"cols":')) ?? "{}");
const top = (app: Awaited<ReturnType<typeof open>>) => Math.min(...app.texts().flatMap(text => text.split("\n")).filter(text => /^SELECT_\d/.test(text)).map(text => Number(text.match(/^SELECT_(\d+)/)![1])));

const grid = (app: Awaited<ReturnType<typeof open>>) => `id:terminal-grid-${app.texts().find(text => text.startsWith("SELECTION_SESSION_"))!.slice("SELECTION_SESSION_".length)}`;

async function ready(app: Awaited<ReturnType<typeof open>>) {
  await app.type("id:selection-terminal", fill);
  await app.keyboard.press("enter");
  await app.waitUntil(() => app.texts().some(text => text.includes("SELECTION_READY")));
  await app.waitFor("label:Terminal history scrollbar");
}

test("dragging beyond the viewport selects history while stationary and stops on release", async () => {
  const app = await open("./fixtures/SelectionFixture.tsx", options);
  try {
    await ready(app);
    const before = top(app);
    await app.dragAndHold(grid(app), { y: -400 });
    await app.waitUntil(() => selected(app).lines > 30 && top(app) < before - 20);
    const progress = top(app);
    await app.waitUntil(() => top(app) < progress - 10);
    await app.drop();
    await app.wait(150);
    const stopped = JSON.stringify(selected(app)), origin = top(app);
    await app.wait(250);
    expect(JSON.stringify(selected(app))).toBe(stopped);
    expect(top(app)).toBe(origin);
  } finally { app.dispose(); }
});

test("the scrollbar navigates the full history and preserves the tracked selection", async () => {
  const app = await open("./fixtures/SelectionFixture.tsx", options);
  try {
    await ready(app);
    await app.drag(grid(app), { x: 35, y: -80 });
    await app.waitUntil(() => selected(app).lines > 1);
    const selection = JSON.stringify(selected(app));
    await app.drag("label:Terminal history scrollbar", { y: -700 });
    await app.waitUntil(() => top(app) < 5);
    expect(JSON.stringify(selected(app))).toBe(selection);
    await app.drag("label:Terminal history scrollbar", { y: 700 });
    await app.waitUntil(() => app.texts().some(text => text.includes("SELECTION_READY")));
    expect(JSON.stringify(selected(app))).toBe(selection);
  } finally { app.dispose(); }
});

test("bottom-edge selection scrolls downward and the alternate screen has no history scrollbar", async () => {
  const app = await open("./fixtures/SelectionFixture.tsx", options);
  try {
    await ready(app);
    await app.drag("label:Terminal history scrollbar", { y: -700 });
    await app.waitUntil(() => top(app) < 5);
    await app.dragAndHold(grid(app), { y: 400 });
    await app.waitUntil(() => selected(app).lines > 30 && top(app) > 20);
    await app.drop();
    await app.type("id:selection-terminal", "printf '\\033[?1049hALT_READY'");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("ALT_READY")));
    expect(app.shows("label:Terminal history scrollbar")).toBe(false);
  } finally { app.dispose(); }
});


test("overlay scrollbar appearing, hovering and fading never change terminal columns", async () => {
  const app = await open("./fixtures/SelectionFixture.tsx", options);
  try {
    await app.waitUntil(() => selected(app).cols > 1);
    const columns = selected(app).cols;
    await ready(app);
    expect(selected(app).cols).toBe(columns);
    await app.hover("label:Terminal history scrollbar");
    await app.wait(100);
    expect(selected(app).cols).toBe(columns);
    await app.hover(grid(app));
    await app.wait(1200);
    expect(selected(app).cols).toBe(columns);
    await app.drag("label:Terminal history scrollbar", { y: -700 });
    await app.waitUntil(() => top(app) < 5);
    expect(selected(app).cols).toBe(columns);
  } finally { app.dispose(); }
});
