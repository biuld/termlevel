import { open, test } from "nsh/test";

test("production preview leaves terminal and Git unavailable", async () => {
  const app = await open("../../App.tsx", {
    platform: "macos", width: 1200, height: 780,
    modules: { "termlevel/native": { "Runtime.previewMode": true } },
  });
  await app.waitFor("text:Terminal sessions are unavailable in preview.");
  await app.press("id:titlebar-git");
  await app.chooseMenuItem("Changes");
  await app.waitFor("text:Git is unavailable in preview.");
  app.dispose();
});
