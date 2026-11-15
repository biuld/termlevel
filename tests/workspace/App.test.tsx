import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1200, height: 780, colorScheme: "dark" as const, modules: { "termlevel/native": { "Runtime.previewMode": true } } };

test("the compact sidebar divider cannot resize or change the saved width", async () => {
  const app = await open("../../App.tsx", { ...options, width: 800 });
  await app.press("id:toggle-sidebar");
  await app.drag("label:Resize terminal sidebar", { x: 200 });
  expect(app.shows("id:session-path")).toBe(true);
  await app.hover("id:sidebar-panel");
  await app.drag("label:Resize terminal sidebar", { x: 200 });
  expect(app.shows("id:session-path")).toBe(true);
  await app.hover("id:session-toolbar");
  await app.press("id:toggle-sidebar");
  expect(app.shows("id:session-path")).toBe(true);
  await app.drag("label:Resize terminal sidebar", { x: 80 });
  expect(app.shows("id:session-path")).toBe(false);
});

test("resizing updates the layout before release and hover still opens the sidebar", async () => {
  const app = await open("../../App.tsx", { ...options, width: 800 });
  await app.type("id:terminal-prototype", "echo resize-overlay-output");
  await app.keyboard.press("enter");
  expect(app.shows("id:session-path")).toBe(true);
  await app.dragAndHold("label:Resize terminal sidebar", { x: 80 });
  expect(app.shows("id:session-path")).toBe(false);
  await app.drop();
  await app.press("id:toggle-sidebar");
  expect(app.shows("id:session-path")).toBe(true);
  await app.hover("id:sidebar-panel");
  await app.hover("id:group-termlevel");
  await app.press("id:group-termlevel");
  await app.waitFor("id:dashboard");
  await app.keyboard.press("escape");
  await app.hover("id:session-toolbar");
  expect(app.shows("id:dashboard")).toBe(false);
  await app.waitFor("text:resize-overlay-output");
  expect(app.count("id:terminal-prototype")).toBe(1);
});

test("the compact rail opens the group overview and keeps terminal output", async () => {
  const app = await open("../../App.tsx", { ...options, width: 640, height: 480 });
  await app.type("id:terminal-prototype", "echo hover-retains-output");
  await app.keyboard.press("enter");
  await app.press("id:toggle-sidebar");
  expect(app.shows("id:session-path")).toBe(true);
  expect(app.shows("id:group-termlevel")).toBe(true);
  await app.hover("id:sidebar-panel");
  await app.press("id:group-termlevel");
  await app.waitFor("id:dashboard");
  await app.hover("id:dashboard");
  expect(app.shows("id:dashboard")).toBe(true);
  expect(app.shows("id:session-path")).toBe(false);
  await app.keyboard.press("escape");
  expect(app.shows("id:dashboard")).toBe(false);
  await app.hover("id:session-toolbar");
  await app.waitFor("text:hover-retains-output");
  expect(app.count("id:terminal-prototype")).toBe(1);
  await app.press("id:toggle-sidebar");
  expect(app.shows("id:window-toolbar")).toBe(true);
});

test("the group chevron toggles the group without opening the overview", async () => {
  const app = await open("../../App.tsx", options);
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:dashboard")).toBe(false);
  expect(app.shows("id:tab-prototype")).toBe(false);
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-prototype")).toBe(true);
});

test("a folded group in the compact rail keeps its active tab and counts hidden tabs", async () => {
  const app = await open("../../App.tsx", options);
  await app.type("id:terminal-prototype", "echo folded-output");
  await app.keyboard.press("enter");
  await app.press("id:toggle-sidebar");
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-prototype")).toBe(true);
  expect(app.shows("id:tab-preview")).toBe(false);
  expect(app.texts()).toContain("+1");
  await app.waitFor("text:folded-output");
  await app.press("id:tab-home");
  expect(app.shows("id:tab-prototype")).toBe(false);
  expect(app.texts()).toContain("+2");
  await app.press("id:group-hidden-termlevel");
  expect(app.shows("id:tab-prototype")).toBe(true);
  expect(app.shows("id:tab-preview")).toBe(true);
  expect(app.shows("id:group-hidden-termlevel")).toBe(false);
  await app.press("id:tab-prototype");
  await app.waitFor("text:folded-output");
  expect(app.count("id:terminal-prototype")).toBe(1);
});

test("switching tabs and collapsing groups preserve terminal output", async () => {
  const app = await open("../../App.tsx", options);
  await app.type("id:terminal-prototype", "echo retained-output");
  await app.keyboard.press("enter");
  await app.waitFor("text:retained-output");
  await app.press("id:tab-preview");
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-prototype")).toBe(false);
  await app.press("id:group-toggle-termlevel");
  await app.press("id:tab-prototype");
  await app.waitFor("text:retained-output");
  expect(app.count("id:terminal-prototype")).toBe(1);
});

test("creates a group and renames its terminal without resetting it", async () => {
  const app = await open("../../App.tsx", options);
  await app.press("id:new-group");
  await app.waitFor("id:new-group-path");
  expect(app.shows("id:dashboard")).toBe(false);
  expect(app.shows("id:terminal-tab-2")).toBe(false);
  await app.keyboard.press("escape");
  expect(app.shows("id:new-group-path")).toBe(false);
  expect(app.shows("id:group-group-1")).toBe(false);
  await app.press("id:new-group");
  await app.press("id:group-path-0");
  await app.waitUntil(() => !app.shows("id:new-group-path"));
  expect(app.shows("id:dashboard")).toBe(false);
  expect(app.shows("id:group-group-1")).toBe(true);
  await app.waitFor("id:terminal-tab-2");
  await app.type("id:terminal-tab-2", "echo still-here");
  await app.keyboard.press("enter");
  await app.rightClick("id:tab-tab-2");
  await app.chooseMenuItem("Rename terminal…");
  await app.clear("id:name-input");
  await app.type("id:name-input", "Deploy");
  await app.press("id:save-name");
  await app.waitFor("text:Deploy");
  await app.waitFor("text:still-here");
});

test("moves a terminal between groups and removes a group without closing terminals", async () => {
  const app = await open("../../App.tsx", options);
  await app.rightClick("id:tab-preview");
  await app.chooseMenuItem("Move to group > Playground");
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-preview")).toBe(true);
  await app.press("id:group-playground");
  await app.waitFor("id:dashboard");
  await app.press("id:dashboard-group-menu");
  await app.chooseMenuItem("Ungroup terminals");
  expect(app.shows("id:group-playground")).toBe(false);
  expect(app.shows("id:tab-preview")).toBe(true);
  expect(app.shows("id:tab-colors")).toBe(true);
  expect(app.shows("id:tab-home")).toBe(true);
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-prototype")).toBe(true);
});

test("closing an active group selects a remaining terminal and an empty workspace can recover", async () => {
  const app = await open("../../App.tsx", options);
  await app.press("id:group-termlevel");
  await app.waitFor("id:dashboard");
  await app.press("id:dashboard-group-menu");
  await app.chooseMenuItem("Delete group…");
  await app.press("id:confirm-close-group");
  expect(app.shows("id:group-termlevel")).toBe(false);
  expect(app.shows("id:tab-home")).toBe(true);
  expect(app.shows("id:tab-colors")).toBe(true);
  expect(app.shows("id:tab-prototype")).toBe(false);
  expect(app.shows("id:tab-preview")).toBe(false);
  await app.keyboard.press("mod+w");
  await app.keyboard.press("mod+w");
  await app.waitFor("id:empty-state");
  await app.press("id:empty-new-terminal");
  await app.waitFor("id:terminal-tab-1");
  expect(app.shows("id:tab-home")).toBe(false);
  expect(app.shows("id:tab-colors")).toBe(false);
});

test("search opens a terminal in a collapsed group and compact sidebar keeps navigation", async () => {
  const app = await open("../../App.tsx", options);
  await app.press("id:group-toggle-playground");
  await app.keyboard.press("mod+k");
  await app.type("id:terminal-search", "Color test");
  await app.press("id:search-colors");
  await app.waitFor("id:tab-colors");
  await app.press("id:toggle-sidebar");
  await app.press("id:tab-home");
  await app.keyboard.press("mod+t");
  await app.waitFor("id:terminal-tab-1");
  await app.press("id:toggle-sidebar");
  expect(app.shows("id:tab-tab-1")).toBe(true);
  expect(app.shows("id:tab-home")).toBe(true);
  expect(app.shows("id:tab-prototype")).toBe(true);
  expect(app.shows("id:tab-preview")).toBe(true);
  expect(app.shows("id:tab-colors")).toBe(true);
});

test("new terminal stays outside groups unless created from a group", async () => {
  const app = await open("../../App.tsx", options);
  await app.press("id:tab-prototype");
  await app.press("id:new-terminal");
  await app.waitFor("id:terminal-tab-1");
  await app.press("id:group-toggle-termlevel");
  await app.press("id:group-toggle-playground");
  expect(app.shows("id:tab-tab-1")).toBe(true);
  expect(app.shows("id:tab-prototype")).toBe(false);
  expect(app.shows("id:tab-colors")).toBe(false);
});

test("pinning and menu ordering preserve the selected session", async () => {
  const app = await open("../../App.tsx", options);
  await app.type("id:terminal-prototype", "echo identity");
  await app.keyboard.press("enter");
  await app.rightClick("id:tab-prototype");
  await app.chooseMenuItem("Pin terminal");
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-prototype")).toBe(true);
  await app.rightClick("id:tab-prototype");
  await app.chooseMenuItem("Move up");
  await app.waitFor("text:identity");
});

test("appearance updates theme tokens without recreating the terminal", async () => {
  const app = await open("../../App.tsx", options);
  await app.type("id:terminal-prototype", "echo theme-keeps-output");
  await app.keyboard.press("enter");
  const darkText = app.spanStyle("id:tab-prototype >> text:UI prototype").color;
  const darkOutput = app.spanStyle("id:terminal-prototype >> text:theme-keeps-output >> nth=1").color;
  const darkGroup = app.spanStyle("id:group-termlevel >> text:termlevel").color;

  await app.press("id:settings");
  await app.press("id:appearance-light");
  await app.press("label:Close settings");
  const lightText = app.spanStyle("id:tab-prototype >> text:UI prototype").color;
  expect(lightText === darkText).toBe(false);
  expect(app.spanStyle("id:terminal-prototype >> text:theme-keeps-output >> nth=1").color === darkOutput).toBe(false);
  expect(app.spanStyle("id:group-termlevel >> text:termlevel").color === darkGroup).toBe(false);
  await app.waitFor("text:theme-keeps-output");

  await app.press("id:settings");
  await app.press("id:appearance-dark");
  await app.press("label:Close settings");
  expect(app.spanStyle("id:tab-prototype >> text:UI prototype").color).toBe(darkText);

  await app.setColorScheme("light");
  // The explicit dark choice overrides system changes.
  expect(app.spanStyle("id:tab-prototype >> text:UI prototype").color).toBe(darkText);
  await app.press("id:settings");
  await app.press("id:appearance-system");
  await app.press("label:Close settings");
  expect(app.spanStyle("id:tab-prototype >> text:UI prototype").color).toBe(lightText);
  await app.waitFor("text:theme-keeps-output");
  expect(app.count("id:terminal-prototype")).toBe(1);
});

test("the session toolbar adapts to narrow windows while keeping its actions accessible", async () => {
  const app = await open("../../App.tsx", options);
  await app.type("id:terminal-prototype", "echo rail-retains-output");
  await app.keyboard.press("enter");
  expect(app.shows("id:session-path")).toBe(true);
  await app.setViewport(640, 480);
  expect(app.shows("id:session-path")).toBe(false);
  await app.press("id:terminal-actions");
  await app.chooseMenuItem("Rename terminal…");
  await app.keyboard.press("escape");
  await app.press("id:toggle-sidebar");
  await app.waitFor("id:session-path");
  expect(app.count("id:session-toolbar")).toBe(1);
  expect(app.shows("id:window-toolbar")).toBe(true);
  await app.waitFor("text:rail-retains-output");
  await app.type("id:terminal-prototype", "echo narrow-window");
  await app.keyboard.press("enter");
  await app.waitFor("text:narrow-window");
  await app.press("id:toggle-sidebar");
  expect(app.shows("id:window-toolbar")).toBe(true);
  expect(app.count("id:session-toolbar")).toBe(1);
  expect(app.count("id:terminal-prototype")).toBe(1);
  await app.waitFor("text:rail-retains-output");
});


test("the overview header changes color and name while keeping terminal output", async () => {
  const app = await open("../../App.tsx", options);
  await app.type("id:terminal-prototype", "echo group-editor-retains-output");
  await app.keyboard.press("enter");
  const before = app.spanStyle("id:group-termlevel >> text:termlevel").color;
  await app.rightClick("id:group-termlevel");
  await app.waitFor("id:dashboard");
  expect(app.value("id:group-name-input")).toBe("termlevel");
  await app.press("id:group-color-trigger");
  for (const color of ["blue", "purple", "cyan", "orange", "yellow", "pink", "green", "gray", "red"]) {
    expect(app.count(`id:group-color-${color}`)).toBe(1);
  }
  await app.press("id:group-color-red");
  expect(app.shows("id:dashboard")).toBe(true);
  expect(app.shows("id:group-termlevel >> label:red group marker")).toBe(true);
  await app.clear("id:group-name-input");
  await app.type("id:group-name-input", "Release");
  // Color belongs to the marker; the name keeps its readable neutral color.
  expect(app.spanStyle("id:group-termlevel >> text:Release").color).toBe(before);
  await app.keyboard.press("escape");
  expect(app.shows("id:dashboard")).toBe(false);
  await app.waitFor("text:group-editor-retains-output");

  await app.press("id:group-termlevel");
  await app.waitFor("id:dashboard");
  await app.press("id:dashboard-new-terminal");
  expect(app.shows("id:dashboard")).toBe(false);
  expect(app.shows("id:tab-tab-1")).toBe(true);
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-tab-1")).toBe(false);
  await app.press("id:group-toggle-termlevel");
  await app.press("id:tab-prototype");
  await app.waitFor("text:group-editor-retains-output");
  expect(app.count("id:terminal-prototype")).toBe(1);
});

test("the group overview opens from the compact rail and closes with Escape", async () => {
  const app = await open("../../App.tsx", options);
  await app.press("id:toggle-sidebar");
  await app.rightClick("id:group-termlevel");
  await app.waitFor("id:dashboard");
  await app.press("id:group-color-trigger");
  await app.press("id:group-color-gray");
  expect(app.shows("id:dashboard")).toBe(true);
  await app.keyboard.press("escape");
  expect(app.shows("id:dashboard")).toBe(false);
  await app.press("id:group-termlevel");
  await app.waitFor("id:dashboard");
  await app.keyboard.press("escape");
  expect(app.shows("id:dashboard")).toBe(false);
  await app.press("id:toggle-sidebar");
  expect(app.count("id:terminal-prototype")).toBe(1);
});

test("dragging a terminal onto a group keeps its session and shows the drop target", async () => {
  const app = await open("../../App.tsx", options);
  await app.press("id:tab-colors");
  await app.type("id:terminal-colors", "echo dragged-into-group");
  await app.keyboard.press("enter");
  await app.waitFor("text:dragged-into-group");
  await app.dragAndHold("id:tab-colors", { y: -130 });
  await app.waitFor("id:drop-group-termlevel");
  await app.drop();
  expect(app.shows("id:drop-group-termlevel")).toBe(false);
  await app.press("id:group-toggle-playground");
  expect(app.shows("id:tab-colors")).toBe(true);
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-colors")).toBe(false);
  await app.press("id:group-toggle-termlevel");
  await app.press("id:tab-colors");
  await app.waitFor("text:dragged-into-group");
  expect(app.count("id:terminal-colors")).toBe(1);
});

test("dragging onto a collapsed group opens it there and a sideways drag does not move the terminal", async () => {
  const app = await open("../../App.tsx", options);
  await app.type("id:terminal-prototype", "echo stayed-put");
  await app.keyboard.press("enter");
  await app.drag("id:tab-prototype", { x: 280 });
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-prototype")).toBe(false);
  await app.press("id:group-toggle-termlevel");
  await app.press("id:group-toggle-playground");
  await app.drag("id:tab-prototype", { y: 70 });
  expect(app.shows("id:tab-prototype")).toBe(true);
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-prototype")).toBe(true);
  await app.waitFor("text:stayed-put");
  expect(app.count("id:terminal-prototype")).toBe(1);
});

test("dragging onto a terminal joins its group, or starts one when it has none", async () => {
  const app = await open("../../App.tsx", options);
  await app.press("id:tab-colors");
  await app.type("id:terminal-colors", "echo grouped-session");
  await app.keyboard.press("enter");
  await app.waitFor("text:grouped-session");
  await app.dragAndHold("id:tab-colors", { y: -100 });
  await app.waitFor("id:drop-tab-prototype");
  await app.drop();
  expect(app.shows("id:drop-tab-prototype")).toBe(false);
  await app.press("id:group-toggle-playground");
  expect(app.shows("id:tab-colors")).toBe(true);
  await app.press("id:group-toggle-termlevel");
  expect(app.shows("id:tab-colors")).toBe(false);
  await app.press("id:group-toggle-termlevel");

  await app.rightClick("id:tab-colors");
  await app.chooseMenuItem("Move to group > No group");
  await app.rightClick("id:tab-preview");
  await app.chooseMenuItem("Move to group > No group");
  await app.dragAndHold("id:tab-colors", { y: 36 });
  await app.waitFor("id:drop-tab-preview");
  await app.drop();
  expect(app.shows("id:dashboard")).toBe(false);
  expect(app.shows("id:group-group-1 >> text:termlevel")).toBe(true);
  await app.press("id:group-group-1");
  await app.waitFor("id:dashboard");
  await app.press("id:dashboard-new-terminal");
  expect(app.texts()).toContain("~/Projects/termlevel");
  await app.press("id:group-toggle-group-1");
  expect(app.shows("id:tab-colors")).toBe(false);
  expect(app.shows("id:tab-preview")).toBe(false);
  await app.press("id:group-toggle-group-1");
  await app.press("id:tab-colors");
  await app.waitFor("text:grouped-session");
  expect(app.count("id:terminal-colors")).toBe(1);
});

test("preview notifications stay empty and the global entry survives sidebar collapse", async () => {
  const app = await open("../../App.tsx", options);
  try {
    await app.press("id:toggle-sidebar");
    await app.press("id:notifications");
    await app.waitFor("id:notifications-empty");
    await app.keyboard.press("escape");
    expect(app.shows("id:notification-panel")).toBe(false);
    expect(app.shows("id:notifications")).toBe(true);
  } finally { app.dispose(); }
});
