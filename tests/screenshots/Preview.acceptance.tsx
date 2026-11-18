import { open, test } from "nsh/test";

const preview = {
  platform: "macos" as const,
  width: 1440,
  height: 900,
  colorScheme: "dark" as const,
};

async function openGit(app: Awaited<ReturnType<typeof open>>, item: string) {
  for (let attempt = 0; attempt < 6; attempt++) {
    await app.press("id:titlebar-git");
    await app.wait(200);
    try {
      await app.chooseMenuItem(item);
      return;
    } catch (error) {
      if (!String(error).includes("no context menu")) throw error;
    }
  }
  throw new Error(`The Git menu did not show ${item}`);
}

test("capture workspace, search and group editing", async () => {
  const app = await open("./tests/screenshots/ScreenshotApp.tsx", preview);
  try {
    await app.waitFor("id:terminal-prototype");
    await app.wait(300);
    await app.screenshot("docs/assets/screenshots/01-workspace-overview.png");

    await app.press("id:pane-find");
    await app.waitFor("id:find-in-terminal");
    await app.type("id:find-query", "termlevel");
    await app.wait(300);
    await app.screenshot("docs/assets/screenshots/02-terminal-find.png");
    await app.press("id:find-close");

    await app.keyboard.press("mod+k");
    await app.waitFor("id:terminal-search");
    await app.type("id:terminal-search", "Color");
    await app.waitFor("id:search-colors");
    await app.screenshot("docs/assets/screenshots/03-search-terminals.png");
    await app.keyboard.press("escape");

    await app.rightClick("id:group-termlevel");
    await app.waitFor("id:group-editor");
    await app.screenshot("docs/assets/screenshots/04-group-editor.png");
    await app.keyboard.press("escape");
  } finally {
    app.dispose();
  }
});

test("capture Git Changes", async () => {
  const app = await open("./tests/screenshots/ScreenshotApp.tsx", preview);
  try {
    await app.waitFor("id:terminal-prototype");
    await openGit(app, "Changes");
    await app.waitFor("id:git-view");
    await app.wait(500);
    await app.screenshot("docs/assets/screenshots/05-git-changes.png");
  } finally {
    app.dispose();
  }
});

test("capture Git History", async () => {
  const app = await open("./tests/screenshots/ScreenshotApp.tsx", preview);
  try {
    await app.waitFor("id:terminal-prototype");
    await openGit(app, "History");
    await app.waitFor("text:Merge feature/tiling");
    await app.wait(500);
    await app.screenshot("docs/assets/screenshots/06-git-history.png");
  } finally {
    app.dispose();
  }
});

test("capture Files", async () => {
  const app = await open("./tests/screenshots/ScreenshotApp.tsx", preview);
  try {
    await app.waitFor("id:terminal-prototype");
    await app.press("id:titlebar-files");
    await app.waitFor("id:files-panel");
    await app.waitFor("id:file-README-md");
    await app.press("id:file-README-md");
    await app.waitFor("id:files-editor");
    await app.wait(400);
    await app.screenshot("docs/assets/screenshots/07-files.png");
  } finally {
    app.dispose();
  }
});

test("capture the collapsed sidebar", async () => {
  const app = await open("./tests/screenshots/ScreenshotApp.tsx", preview);
  try {
    await app.waitFor("id:terminal-prototype");
    await app.press("id:toggle-sidebar");
    await app.hover("id:sidebar-panel");
    await app.wait(300);
    await app.screenshot("docs/assets/screenshots/09-sidebar-collapsed.png");
  } finally {
    app.dispose();
  }
});

test("capture settings and the light theme", async () => {
  const app = await open("./tests/screenshots/ScreenshotApp.tsx", preview);
  try {
    await app.waitFor("id:terminal-prototype");
    await app.press("id:settings");
    await app.waitFor("id:appearance-dark");
    await app.wait(300);
    await app.screenshot("docs/assets/screenshots/10-settings-dark.png");

    await app.press("id:appearance-light");
    await app.wait(400);
    await app.screenshot("docs/assets/screenshots/11-settings-light.png");

    await app.press("label:Close settings");
    await app.wait(400);
    await app.screenshot("docs/assets/screenshots/12-workspace-light.png");
  } finally {
    app.dispose();
  }
});
