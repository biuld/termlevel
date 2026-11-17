import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1400, height: 800, colorScheme: "dark" as const };

test("Git scale: loaded commits and refs create only viewport rows and unchanged refs retain history", async () => {
  const app = await open("./fixtures/GitScaleFixture.tsx", options);
  try {
    await app.waitFor("text:Scale commit 0");
    const rendered = () => app.texts().filter(text => /^Scale commit \d+$/.test(text));
    expect(rendered().length < 100).toBe(true);
    expect(app.texts().includes("Scale commit 3999")).toBe(false);
    await app.scroll("id:history-commits", 3000);
    await app.waitUntil(() => !app.shows("id:history-commit-0000000"));
    expect(rendered().length < 100).toBe(true);
    await app.press("id:scale-refresh");
    await app.waitUntil(() => app.texts().some(text => text.includes("Snapshots 1 · Unchanged 1 · Logs 1")));
    expect(app.shows("id:history-commit-0000000")).toBe(false);
  } finally { app.dispose(); }
});

test("Git scale: search covers all loaded commits while hiding disconnected graph edges", async () => {
  const app = await open("./fixtures/GitScaleFixture.tsx", options);
  try {
    await app.waitFor("text:Scale commit 0");
    await app.press("id:history-search-open");
    await app.waitFor("id:history-search");
    await app.paste("id:history-search", { text: "Scale commit 3999" });
    await app.waitFor("text:Scale commit 3999");
    await app.waitFor("text:1/4000");
    await app.waitFor("text:Connections are hidden while filtering commits.");
    expect(app.shows("id:history-commit-0000000")).toBe(false);
  } finally { app.dispose(); }
});

test("Git scale: changing repositories rejects late log results and closing releases the read lease", async () => {
  const app = await open("./fixtures/GitStaleReadFixture.tsx", options);
  try {
    await app.waitUntil(() => app.texts().some(text => text.includes("Pending true")));
    await app.press("id:switch-repository");
    await app.waitUntil(() => app.texts().includes("/beta Merge feature/history"));
    await app.press("id:release-old");
    await app.waitUntil(() => app.texts().some(text => text.includes("Pending false")));
    expect(app.texts().some(text => text.startsWith("/alpha "))).toBe(false);
    await app.press("id:close-history");
    await app.waitUntil(() => app.texts().some(text => text.includes("Disposed 1")));
  } finally { app.dispose(); }
});
