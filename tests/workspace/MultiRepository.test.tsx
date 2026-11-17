import { expect, open, test } from "nsh/test";
const options = { platform: "macos" as const, width: 1300, height: 800, colorScheme: "dark" as const };
async function choose(app: Awaited<ReturnType<typeof open>>, root: string) {
  await app.press("id:git-repository-selector"); await app.press(`id:git-repository-${root}`);
}

test("multiple repositories keep manual choices per group and follow cwd only when requested", async () => {
  const app = await open("./fixtures/MultiRepositoryFixture.tsx", options);
  try {
    await app.waitFor("text:alpha commit 0");
    await choose(app, "/work/alpha");
    await app.press("id:cd-beta");
    await app.waitFor("text:alpha commit 0");
    await app.press("id:git-repository-selector"); await app.press("id:git-follow-context");
    await app.waitFor("text:beta commit 0");
    await choose(app, "/work/alpha");
    await app.press("id:group-two"); await app.waitFor("text:gamma commit 0");
    await app.press("id:group-one"); await app.waitFor("text:alpha commit 0");
    await app.press("id:hide-git"); await app.press("id:hide-git");
    await app.waitFor("text:alpha commit 0");
  } finally { app.dispose(); }
});

test("repository history restores its search, selected commit and native read cursor", async () => {
  const app = await open("./fixtures/MultiRepositoryFixture.tsx", options);
  try {
    await app.waitFor("text:alpha commit 0");
    await app.press("id:history-search-open");
    await app.paste("id:history-search", { text: "alpha commit 17" });
    await app.waitFor("text:11/200");
    await app.press("id:history-commit-17");
    await choose(app, "/work/beta"); await app.waitFor("text:beta commit 0");
    await choose(app, "/work/alpha");
    await app.waitFor("text:11/200");
    await app.waitFor("id:history-detail-wide >> text:alpha commit 17");
    await app.waitUntil(() => app.texts().some(text => text.includes("Alpha logs 1 · Beta logs 1 · Resumes")));
    await app.press("id:history-search-close");
    await app.scroll("id:history-commits", 6000);
    await app.press("id:history-load-more");
    await app.waitUntil(() => app.texts().some(text => text.includes("Alpha logs 2")));
  } finally { app.dispose(); }
});

test("missing selected repositories do not silently select another repository", async () => {
  const app = await open("./fixtures/MultiRepositoryFixture.tsx", options);
  try {
    await app.waitFor("text:alpha commit 0");
    await choose(app, "/work/beta"); await app.waitFor("text:beta commit 0");
    await app.press("id:remove-beta");
    await app.waitFor("text:Selected repository is unavailable.");
    expect(app.shows("id:history-panel")).toBe(false);
    await app.press("id:remove-beta");
    await app.waitFor("text:beta commit 0");
  } finally { app.dispose(); }
});

test("a large repository picker is searchable and only renders viewport rows", async () => {
  const app = await open("./fixtures/MultiRepositoryFixture.tsx", options);
  try {
    await app.press("id:group-many"); await app.waitFor("text:Select a repository.");
    await app.press("id:git-repository-selector");
    expect(app.texts().filter(text => /^project-\d+$/.test(text)).length < 60).toBe(true);
    await app.paste("id:git-repository-search", { text: "project-1499" });
    await app.waitFor("text:project-1499");
    await app.press("id:git-repository-/many/project-1499");
    await app.waitFor("text:project-1499 commit 0");
  } finally { app.dispose(); }
});
