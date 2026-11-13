import { expect, open, test } from "nsh/test";
import { equalize, geometry, insertPane, minimum, neighbor, panes, removePane, type LayoutNode } from "./tiling";

function state(app: Awaited<ReturnType<typeof open>>): { tiling: { root: LayoutNode; focusedPaneId: string }; activeId: string } {
  return JSON.parse(app.texts().find(t => t.startsWith('{"tiling":')) ?? "null");
}

const options = { platform: "macos" as const, width: 1400, height: 900, colorScheme: "dark" as const, modules: { "termlevel/native": { "Runtime.previewMode": true } } };

test("nested tiling geometry clamps sizes, normalizes removals and finds directional neighbors", () => {
  let root: LayoutNode = { kind: "pane", id: "a", sessionId: "s-a" };
  root = insertPane(root, "a", { kind: "pane", id: "b", sessionId: "s-b" }, "right", "ab");
  root = insertPane(root, "b", { kind: "pane", id: "c", sessionId: "s-c" }, "down", "bc");
  const min = { width: 300, height: 200 };
  expect(minimum(root, min, 8)).toEqual({ width: 608, height: 408 });
  const { frames } = geometry(root, { x: 0, y: 0, width: 1000, height: 700 }, min, 8);
  expect(neighbor("b", "down", frames)).toBe("c");
  expect(neighbor("c", "left", frames)).toBe("a");
  expect(neighbor("a", "left", frames)).toBe(null);
  root = removePane(root, "b")!;
  expect(panes(root).map(p => p.sessionId)).toEqual(["s-a", "s-c"]);
  expect(equalize(root).kind).toBe("split");
  expect(removePane(removePane(root, "a"), "c")).toBe(null);
});

test("split panes retain output, replace only focused slots, maximize, hide and restore at narrow widths", async () => {
  const app = await open("../App.tsx", options);
  await app.type("id:terminal-prototype", "echo LEFT_KEPT"); await app.keyboard.press("enter");
  await app.press("id:split-right");
  await app.waitFor("id:terminal-tab-1");
  expect(app.shows("id:terminal-prototype")).toBe(true);
  await app.type("id:terminal-tab-1", "echo RIGHT_KEPT"); await app.keyboard.press("enter");
  await app.keyboard.press("mod+shift+d");
  await app.waitFor("id:terminal-tab-2");
  expect(app.shows("id:terminal-prototype")).toBe(true);
  expect(app.shows("id:terminal-tab-1")).toBe(true);
  await app.press("id:tab-home");
  expect(app.shows("id:terminal-home")).toBe(true);
  expect(app.shows("id:terminal-tab-2")).toBe(false);
  await app.press("id:tab-prototype");
  expect(app.shows("id:terminal-home")).toBe(false);
  expect(app.shows("id:terminal-tab-1")).toBe(true);
  expect(app.shows("id:terminal-tab-2")).toBe(true);
  await app.press("id:zoom-pane");
  expect(app.shows("id:terminal-tab-1")).toBe(false);
  await app.press("id:zoom-pane");
  await app.waitFor("text:RIGHT_KEPT");
  await app.setViewport(640, 480);
  expect(app.shows("id:layout-hint")).toBe(true);
  expect(app.shows("id:terminal-tab-1")).toBe(false);
  await app.setViewport(1400, 900);
  await app.waitFor("text:RIGHT_KEPT");
  await app.press("id:terminal-actions"); await app.chooseMenuItem("Hide pane");
  expect(app.shows("id:terminal-prototype")).toBe(false);
  await app.press("id:tab-prototype");
  await app.waitFor("text:LEFT_KEPT");
  expect(app.count("id:terminal-prototype")).toBe(1);
});

test("directional focus, swap, live divider resize and resize-mode keys preserve terminals", async () => {
  const app = await open("../App.tsx", options);
  await app.press("id:split-right");
  await app.type("id:terminal-tab-1", "echo SWAP_KEPT"); await app.keyboard.press("enter");
  await app.keyboard.press("mod+alt+left");
  expect(app.shows("id:pane-prototype >> id:session-toolbar")).toBe(true);
  await app.keyboard.press("mod+alt+shift+right");
  expect(app.shows("id:pane-prototype >> id:session-toolbar")).toBe(true);
  await app.type("id:terminal-prototype", "echo FOCUS_FOLLOWS_SESSION"); await app.keyboard.press("enter");
  await app.waitFor("text:FOCUS_FOLLOWS_SESSION");
  await app.dragAndHold("label:Resize terminal panes", { x: 60 });
  await app.waitFor("text:SWAP_KEPT"); await app.drop();
  await app.press("id:terminal-actions"); await app.chooseMenuItem("Resize with arrow keys");
  await app.keyboard.press("left"); await app.keyboard.press("enter");
  expect(app.shows("id:layout-hint")).toBe(false);
  await app.type("id:terminal-prototype", "echo KEYS_RELEASED"); await app.keyboard.press("enter");
  await app.waitFor("text:KEYS_RELEASED");
  expect(app.count("id:terminal-tab-1")).toBe(1);
});

test("sidebar dragging to a pane creates a split and cancellation does not change sessions", async () => {
  const app = await open("./fixtures/TilingStateFixture.tsx", options);
  await app.dragAndHold("id:tab-colors", { x: 1230, y: 100 });
  await app.waitFor("id:pane-drop-preview");
  expect(app.shows("text:Add to this group")).toBe(true);
  await app.drop();
  expect(app.shows("id:terminal-colors")).toBe(true);
  expect(app.shows("id:terminal-prototype")).toBe(true);
  expect(app.shows("id:tab-tab-1")).toBe(false);
  await app.press("id:group-termlevel");
  expect(app.shows("id:tab-colors")).toBe(false);
  await app.press("id:group-termlevel");
  const before = JSON.stringify(state(app).tiling.root);
  await app.dragAndHold("id:tab-home", { x: 600, y: 80 });
  await app.keyboard.press("escape"); await app.drop();
  expect(app.shows("id:pane-drop-preview")).toBe(false);
  expect(JSON.stringify(state(app).tiling.root)).toBe(before);
  expect(panes(state(app).tiling.root).map(p => p.sessionId)).toEqual(["prototype", "colors"]);
  expect(app.count("id:terminal-prototype")).toBe(1);
});

test("too-small split is rejected without starting a terminal", async () => {
  const app = await open("../App.tsx", { ...options, width: 640, height: 480 });
  await app.keyboard.press("mod+d");
  expect(app.shows("id:tab-tab-1")).toBe(false);
  expect(app.shows("id:terminal-tab-1")).toBe(false);
  expect(app.shows("id:layout-hint")).toBe(true);
});

test("each tab group keeps its own tiling layout", async () => {
  const app = await open("../App.tsx", options);
  expect(app.shows("id:group-workspace-termlevel")).toBe(true);
  await app.press("id:split-right");
  await app.press("id:tab-colors");
  expect(app.shows("id:group-workspace-playground")).toBe(true);
  expect(app.shows("id:terminal-colors")).toBe(true);
  expect(app.shows("id:terminal-prototype")).toBe(false);
  expect(app.shows("id:terminal-tab-1")).toBe(false);
  await app.press("id:tab-prototype");
  expect(app.shows("id:terminal-prototype")).toBe(true);
  expect(app.shows("id:terminal-tab-1")).toBe(true);
  expect(app.shows("id:terminal-colors")).toBe(false);
  await app.press("id:group-menu-termlevel"); await app.press("id:group-editor-delete"); await app.press("id:confirm-close-group");
  expect(app.shows("id:terminal-prototype")).toBe(false);
  expect(app.shows("id:terminal-tab-1")).toBe(false);
  expect(app.shows("id:terminal-colors")).toBe(true);
  expect(app.shows("id:pane-colors >> id:session-toolbar")).toBe(true);
  expect(app.shows("id:tab-home")).toBe(true);
  expect(app.shows("id:tab-colors")).toBe(true);
  expect(app.shows("id:tab-prototype")).toBe(false);
  expect(app.shows("id:tab-preview")).toBe(false);
});

test("a split is its own sidebar unit and can move into a tab group", async () => {
  const app = await open("../App.tsx", options);
  await app.press("id:tab-home");
  await app.press("id:split-right");
  await app.waitFor("id:split-unit");
  expect(app.shows("id:split-unit >> id:tab-home")).toBe(true);
  expect(app.shows("id:split-unit >> id:tab-tab-1")).toBe(true);
  expect(app.shows("label:Shown in the layout")).toBe(true);
  expect(app.shows("id:group-termlevel")).toBe(true);
  expect(app.shows("id:split-unit-termlevel")).toBe(false);
  expect(app.shows("id:split-menu")).toBe(false);
  await app.rightClick("id:tab-home");
  await app.chooseMenuItem("Move split to group > Playground");
  expect(app.shows("id:split-unit")).toBe(false);
  expect(app.shows("id:split-unit-playground >> id:tab-home")).toBe(true);
  expect(app.shows("id:split-unit-playground >> id:tab-tab-1")).toBe(true);
  await app.press("id:group-playground");
  expect(app.shows("id:tab-home")).toBe(false);
  expect(app.shows("id:tab-tab-1")).toBe(false);
  await app.press("id:tab-prototype");
  expect(app.shows("id:terminal-prototype")).toBe(true);
  expect(app.shows("id:terminal-home")).toBe(false);
});

test("a sidebar choice outside the split replaces the whole canvas", async () => {
  const app = await open("../App.tsx", options);
  await app.press("id:split-right");
  await app.press("id:tab-preview");
  expect(app.shows("id:terminal-preview")).toBe(true);
  expect(app.shows("id:terminal-prototype")).toBe(false);
  expect(app.shows("id:terminal-tab-1")).toBe(false);
  expect(app.shows("id:split-unit-termlevel >> id:tab-prototype")).toBe(true);
  expect(app.shows("id:split-unit-termlevel >> id:tab-tab-1")).toBe(true);
  await app.press("id:tab-prototype");
  expect(app.shows("id:terminal-prototype")).toBe(true);
  expect(app.shows("id:terminal-tab-1")).toBe(true);
  expect(app.shows("id:terminal-preview")).toBe(false);
});

test("ctrl+d closes a clean shell and a failed exit keeps the terminal", async () => {
  const app = await open("../App.tsx", options);
  await app.press("id:split-right");
  await app.type("id:terminal-tab-1", "exit 1");
  await app.keyboard.press("enter");
  await app.waitFor("id:session-status-tab-1");
  expect(app.shows("id:tab-tab-1")).toBe(true);
  await app.press("id:terminal-prototype");
  await app.keyboard.press("ctrl+d");
  await app.waitUntil(() => !app.shows("id:tab-prototype"));
  expect(app.shows("id:terminal-tab-1")).toBe(true);
  expect(app.shows("id:session-status-tab-1")).toBe(true);
  expect(app.shows("id:tab-tab-1")).toBe(true);
});

test("pane-title edge dragging moves an existing pane and center dragging swaps without changing sessions", async () => {
  const app = await open("./fixtures/TilingStateFixture.tsx", options);
  await app.press("id:split-right");
  await app.dragAndHold("id:pane-title-tab-1", { x: -580, y: 780 });
  await app.waitFor("id:pane-drop-preview");
  expect(app.shows("text:Move terminal here")).toBe(true); await app.drop();
  const moved = state(app).tiling.root;
  expect(moved.kind === "split" && moved.axis === "column").toBe(true);
  expect(app.shows("id:terminal-prototype")).toBe(true);
  expect(app.shows("id:terminal-tab-1")).toBe(true);
  expect(app.shows("id:tab-home")).toBe(true);
  expect(app.shows("id:tab-preview")).toBe(true);
  expect(app.shows("id:tab-colors")).toBe(true);
  await app.dragAndHold("id:pane-title-prototype", { x: 0, y: 550 });
  await app.waitFor("id:pane-drop-preview"); await app.drop();
  expect(panes(state(app).tiling.root).map(p => p.sessionId)).toEqual(["tab-1", "prototype"]);
  expect(state(app).activeId).toBe("prototype");
  expect(app.count("id:terminal-prototype")).toBe(1);
  expect(app.count("id:terminal-tab-1")).toBe(1);
});
