import { expect, open, test } from "nsh/test";

test("Git results appear as temporary toasts and remain in global notification history", async () => {
  const app = await open("./fixtures/GitActionsFixture.tsx", { platform: "macos", width: 1100, height: 750 });
  try {
    await app.waitFor("id:history-fetch");
    await app.press("id:history-fetch");
    await app.press("id:workspace-ungrouped");
    await app.waitFor("id:notification-toast");
    expect(app.texts()).toContain("Fetch complete");
    await app.waitFor("id:notifications-unread");
    await app.waitUntil(() => !app.shows("id:notification-toast"), { timeout: 7000 });
    await app.press("id:group-termlevel");
    await app.waitFor("id:history-pull");
    await app.press("id:history-pull");
    await app.waitFor("id:notification-toast");
    expect(app.texts()).toContain("Pull failed");
    await app.press("id:toast-dismiss");
    expect(app.shows("id:notification-toast")).toBe(false);
    await app.press("id:history-push");
    await app.waitFor("id:notification-toast");
    expect(app.texts()).toContain("Push complete");
    await app.press("id:toast-dismiss");
    await app.press("id:notifications");
    await app.waitFor("id:notification-panel");
    expect(app.texts().some(text => text.includes("Cannot fast-forward divergent branches."))).toBe(true);
    expect(app.texts().some(text => text.includes("Fetch complete"))).toBe(true);
    expect(app.texts().some(text => text.includes("Push complete"))).toBe(true);
  } finally { app.dispose(); }
});
