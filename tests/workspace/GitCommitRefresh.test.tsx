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
