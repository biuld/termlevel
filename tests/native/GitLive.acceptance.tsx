import { open, test } from "nsh/test";

test("the overview reads the real repository through the system git CLI", async () => {
  const app = await open("./tests/workspace/fixtures/GitLiveFixture.tsx", {
    platform: "macos", width: 1200, height: 780, colorScheme: "dark",
  });
  try {
    await app.waitFor("id:dashboard");
    await app.waitUntil(() =>
      app.texts().some((text) => text === "Changed files" || text === "Working tree clean"));
    await app.wait(600);
    await app.screenshot("artifacts/dashboard-git-live.png");
  } finally {
    app.dispose();
  }
});
