import { open, test } from "nsh/test";

test("remote actions update the visible commit list immediately", async () => {
  const app = await open("./fixtures/GitCommitRefreshFixture.tsx", { platform: "macos", width: 1100, height: 750 });
  try {
    await app.waitFor("text:Merge feature/history");
    for (const action of ["fetch", "pull", "push"]) {
      await app.press(`id:history-${action}`);
      await app.waitFor(`text:New commit after ${action}`);
      await app.waitFor("id:toast-dismiss");
      await app.press("id:toast-dismiss");
    }
  } finally { app.dispose(); }
});

test("push moves remote decorations on existing rows and the selected detail", async () => {
  const app = await open("./fixtures/GitRefsRefreshFixture.tsx", { platform: "macos", width: 1400, height: 800 });
  try {
    await app.waitFor("id:history-commit-row-2b6f0d9 >> text:origin/main");
    await app.waitFor("id:history-detail-wide >> text:main");
    await app.press("id:history-push");
    await app.waitFor("id:history-commit-row-9f3c1a2 >> text:origin/main");
    await app.waitUntil(() => !app.shows("id:history-commit-row-2b6f0d9 >> text:origin/main"));
    await app.waitFor("id:history-detail-wide >> text:origin/main");
  } finally { app.dispose(); }
});

test("a scheduled refresh updates HEAD decorations without changing ref targets", async () => {
  const app = await open("./fixtures/GitRefsRefreshFixture.tsx", { platform: "macos", width: 1400, height: 800 });
  try {
    await app.waitFor("id:history-detail-wide >> text:main");
    await app.press("id:detach-head");
    await app.waitFor("id:history-commit-row-9f3c1a2 >> text:HEAD");
    await app.waitFor("id:history-detail-wide >> text:HEAD");
  } finally { app.dispose(); }
});

test("push clears moved refs from an older selected commit without changing selection", async () => {
  const app = await open("./fixtures/GitRefsRefreshFixture.tsx", { platform: "macos", width: 1400, height: 800 });
  try {
    await app.waitFor("id:history-commit-2b6f0d9");
    await app.press("id:history-commit-2b6f0d9");
    await app.waitFor("id:history-detail-wide >> text:2b6f0d9");
    await app.waitFor("id:history-detail-wide >> text:origin/main");
    await app.press("id:history-push");
    await app.waitFor("id:history-commit-row-9f3c1a2 >> text:origin/main");
    await app.waitUntil(() => !app.shows("id:history-detail-wide >> text:origin/main"));
    await app.waitFor("id:history-detail-wide >> text:2b6f0d9");
  } finally { app.dispose(); }
});
