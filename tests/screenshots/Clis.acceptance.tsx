import { open, test } from "nsh/test";

// The hero capture: UNGROUPED split panes running classic CLI programs, with
// an Agents tab group below them. The group sessions report OSC 7501 idle
// identities themselves; no agent binary, account or private session is used.
test("capture ungrouped split panes and an agent tab group", async () => {
  const app = await open("./tests/screenshots/HeroApp.tsx", {
    platform: "macos", width: 1440, height: 900, colorScheme: "dark",
  });
  const ids = () => (app.texts().find((text) => text.startsWith("ids:"))?.slice(4).split(",").filter(Boolean) ?? []);
  const mapping = () => app.texts().find((text) => text.startsWith("u:"))?.slice(2) ?? "";
  try {
    await app.waitFor("id:session-panel");
    await app.waitFor("id:screenshot-ids");
    await app.waitFor("text:❯");
    const [ungrouped, agentsPart] = mapping().split(";a:");
    const agents = agentsPart.split(",");

    for (const [id, name, appKey] of [
      [agents[0], "Claude", "claude-code"],
      [agents[1], "Codex", "codex"],
      [agents[2], "Pi", "pi"],
    ] as const) {
      await app.press(`id:tab-${id}`);
      await app.waitFor(`id:terminal-${id}`);
      await app.type(`id:terminal-${id}`, `printf '\\033]0;${name}\\007\\033]7501;state=idle:app=${appKey}\\007'; clear`);
      await app.keyboard.press("enter");
      await app.wait(250);
    }

    await app.press(`id:tab-${ungrouped}`);
    await app.waitFor(`id:terminal-${ungrouped}`);
    await app.type(`id:terminal-${ungrouped}`, "printf '\\033]0;Vim\\007'; clear; vim -u NONE -c 'set number' README.md");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some((text) => text.includes("README.md")));
    await app.wait(400);

    const known = new Set([ungrouped, ...agents]);
    await app.press("id:split-right");
    await app.waitUntil(() => ids().some((id) => !known.has(id)));
    const second = ids().find((id) => !known.has(id))!;
    known.add(second);
    await app.waitFor(`id:terminal-${second}`);
    await app.type(`id:terminal-${second}`, "printf '\\033]0;Less\\007'; clear; less NOTES.md");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some((text) => text.includes("Release notes")));
    await app.wait(400);

    await app.press("id:split-down");
    await app.waitUntil(() => ids().some((id) => !known.has(id)));
    const third = ids().find((id) => !known.has(id))!;
    await app.waitFor(`id:terminal-${third}`);
    await app.type(`id:terminal-${third}`, "printf '\\033]0;Matrix\\007'; clear; /opt/homebrew/bin/cmatrix -C cyan");
    await app.keyboard.press("enter");
    await app.wait(900);
    await app.hover("id:window-toolbar");
    await app.wait(200);

    await app.screenshot("docs/assets/screenshots/08-tiling-cli.png");
  } finally {
    try { await app.window("main").pressClose(); }
    catch (error) { if (!String(error).includes("the host left")) throw error; }
    app.dispose();
  }
});
