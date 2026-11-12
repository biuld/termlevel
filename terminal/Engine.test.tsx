import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 800, height: 600 };

test("the native provider actually installs Alacritty and preserves the primary screen", async () => {
  const app = await open("./fixtures/EngineFixture.tsx", options);
  await app.waitFor("text:QQQQQ");
  expect(app.spanStyle("id:engine-terminal >> text:中文é").color).toBe("#0c2238ff");
  await app.press("id:engine-alternate");
  await app.waitFor("text:alternate");
  expect(app.shows("text:QQQQQ")).toBe(false);
  await app.press("id:engine-primary");
  await app.waitFor("text:QQQQQ");
  await app.setViewport(500, 400);
  await app.waitFor("text:QQQQQ");
  expect(app.spanStyle("id:engine-terminal >> text:中文é").color).toBe("#0c2238ff");
});

test("Alacritty modes and protocol replies reach NSH input events", async () => {
  const app = await open("./fixtures/EngineFixture.tsx", options);
  await app.press("id:engine-modes");
  await app.keyboard.press("up");
  expect(app.texts()).toContain(JSON.stringify("\x1bOA"));
  await app.paste("id:engine-terminal", { text: "hello" });
  expect(app.texts()).toContain(JSON.stringify("\x1b[200~hello\x1b[201~"));
  await app.press("id:engine-query");
  await app.waitFor("text:Alacritty test");
  expect(app.texts().some((text) => text.startsWith('"\\u001b[') && text.endsWith('R"'))).toBe(true);
});
