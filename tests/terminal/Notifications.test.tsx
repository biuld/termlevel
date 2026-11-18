import { expect, open, test } from "nsh/test";

const options = { platform: "macos" as const, width: 1100, height: 750 };
async function close(app: Awaited<ReturnType<typeof open>>) {
  try { await app.window("main").pressClose(); }
  catch (error) { if (!String(error).includes("the host left")) throw error; }
  app.dispose();
}

test("background terminal notifications reach the panel and navigate to their source", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.type("id:session-panel", "sleep 0.5; printf '\\033]777;notify;Build finished;All tests passed\\007'");
    await app.keyboard.press("enter");
    await app.keyboard.press("mod+t");
    await app.waitFor("id:notifications-unread");
    await app.press("id:toggle-sidebar");
    await app.press("id:notifications");
    await app.waitFor("id:notification-panel");
    expect(app.texts()).toContain("Build finished");
    expect(app.texts()).toContain("All tests passed");
    expect(app.shows("id:notifications-unread")).toBe(false);
    await app.press("label:Open source of Build finished");
    expect(app.shows("id:notification-panel")).toBe(false);
    // Returning to the source must retain its shell and output.
    await app.type("id:session-panel", "printf 'SOURCE_%s\\n' ALIVE");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("SOURCE_ALIVE")));
    await app.keyboard.press("mod+w");
    await app.press("id:notifications");
    expect(app.texts()).toContain("Build finished");
    expect(app.texts()).toContain("Terminal closed");
    await app.press("id:notifications-clear");
    await app.waitFor("id:notifications-empty");
    await app.keyboard.press("escape");
    expect(app.shows("id:notification-panel")).toBe(false);
  } finally { await close(app); }
});

test("app input warnings appear only in notifications while terminal input keeps working", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.paste("id:session-panel", { text: "x".repeat(1024 * 1024 + 1) });
    await app.waitFor("id:notifications-unread");
    expect(app.texts().some(text => text.includes("Input not sent:"))).toBe(false);
    await app.type("id:session-panel", "printf 'RECOVER_%s\\n' OK");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("RECOVER_OK")));
    await app.press("id:notifications");
    await app.waitFor("text:Input not sent");
    expect(app.texts().some(text => text.includes("Termlevel · Warning"))).toBe(true);
    await app.press("label:Dismiss Input not sent notification");
    await app.waitUntil(() => !app.texts().some(text => text === "Input not sent"));
  } finally { await close(app); }
});

test("notification history scrolls after three entries and preserves both OSC forms", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.type("id:session-panel", "for n in 1 2 3 4 5; do printf '\\033]9;Notice %s\\007' $n; done");
    await app.keyboard.press("enter");
    await app.waitFor("id:notifications-unread");
    await app.setViewport(640, 420);
    await app.press("id:notifications");
    await app.waitUntil(() => app.texts().some(text => text.includes("5 / 100")));
    expect(app.texts()).toContain("Notice 5");
    expect(app.texts().some(text => text.includes("Scroll for more"))).toBe(true);
    await app.scroll("id:notifications-history", 600);
    await app.waitFor("text:Notice 1");
    await app.press("id:notifications-clear");
    await app.waitFor("id:notifications-empty");
  } finally { await close(app); }
});

test("an exited pane keeps its output and offers a new terminal without a bottom status bar", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.type("id:session-panel", "printf 'SAVED_%s\\n' OUTPUT; exit 7");
    await app.keyboard.press("enter");
    await app.waitFor("text:Exited · 7");
    expect(app.texts().some(text => text.includes("SAVED_OUTPUT"))).toBe(true);
    expect(app.texts().some(text => text.includes("Process exited with code"))).toBe(false);
    await app.press("id:notifications");
    await app.waitFor("text:Terminal exited");
    expect(app.texts()).toContain("Process exited with code 7.");
    await app.press("id:notifications-clear");
    await app.keyboard.press("escape");
    expect(app.shows("text:Exited · 7")).toBe(true);
    await app.press("label:Open new terminal");
    await app.type("label:Terminal Terminal 2", "printf 'FRESH_%s\\n' SHELL");
    await app.keyboard.press("enter");
    await app.waitUntil(() => app.texts().some(text => text.includes("FRESH_SHELL")));
  } finally { await close(app); }
});


test("program notifications lead with the request and expose semantic icons", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.type("id:session-panel", "printf '\\033]7501;state=blocked:kind=permission:app=agent:msg=QXBwbHk/\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("id:notifications-unread");
    await app.press("id:notifications");
    await app.waitFor("label:Needs approval");
    expect(app.texts()).toContain("Apply?");
    expect(app.texts()).toContain("Terminal · agent · Needs approval");
    expect(app.texts().some(text => text.startsWith("Needs approval ·"))).toBe(false);
    await app.press("label:Open source of Needs approval · Terminal");
    expect(app.shows("id:notification-panel")).toBe(false);
    // Opening the source acknowledges the reminder, but does not resolve it.
    await app.waitFor("label:Terminal 1, Needs approval, agent · Apply?");
  } finally { await close(app); }
});

test("each notification keeps the terminal title from the moment it arrived", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.type("id:session-panel", "printf '\\033]0;SOURCE_ALPHA\\007\\033]777;notify;Alpha done;From alpha\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("id:notifications-unread");
    await app.keyboard.press("mod+t");
    await app.waitFor("text:Terminal 2");
    await app.type("label:Terminal Terminal 2", "printf '\\033]0;SOURCE_BETA\\007\\033]777;notify;Beta done;From beta\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("text:SOURCE_BETA");
    await app.press("id:notifications");
    await app.waitFor("text:SOURCE_ALPHA");
    await app.press("label:Open source of Alpha done");
    expect(app.shows("id:notification-panel")).toBe(false);
    await app.type("id:session-panel", "printf '\\033]0;SOURCE_LATER\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("text:SOURCE_LATER");
    await app.press("id:notifications");
    await app.waitFor("text:SOURCE_ALPHA");
    expect(app.texts()).toContain("SOURCE_BETA");
    expect(app.texts().filter(text => text.includes("SOURCE_LATER")).length).toBeGreaterThan(0);
    expect(app.texts().some(text => text.includes("SOURCE_ALPHA") && text.includes("SOURCE_LATER"))).toBe(false);
  } finally { await close(app); }
});

test("closing a terminal keeps its remembered title and disables the source link", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  try {
    await app.type("id:session-panel", "printf '\\033]0;SOURCE_GONE\\007\\033]777;notify;Gone done;From gone\\007'");
    await app.keyboard.press("enter");
    await app.waitFor("id:notifications-unread");
    await app.keyboard.press("mod+t");
    await app.waitFor("text:Terminal 2");
    await app.press("id:notifications");
    await app.press("label:Open source of Gone done");
    await app.keyboard.press("mod+w");
    await app.press("id:notifications");
    await app.waitFor("text:SOURCE_GONE · Closed");
    expect(app.texts()).not.toContain("Terminal closed");
    expect(app.shows("id:notification-panel")).toBe(true);
  } finally { await close(app); }
});

test("a long terminal title stays inside the row and the dismiss control remains usable", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  const title = "Thinking ".repeat(24) + "GROK_END";
  try {
    await app.type("id:session-panel", `printf '\\033]0;%s\\007\\033]777;notify;Grok;Turn complete\\007' '${title}'`);
    await app.keyboard.press("enter");
    await app.waitFor("id:notifications-unread");
    await app.press("id:notifications");
    await app.waitFor("text:Turn complete");
    await app.press("label:Dismiss Grok notification");
    await app.waitUntil(() => !app.texts().some(text => text === "Turn complete"));
  } finally { await close(app); }
});

test("long notification content expands in place and remains available after collapse", async () => {
  const app = await open("./fixtures/NotificationFixture.tsx", options);
  const message = "Diagnostic detail. ".repeat(80) + "DETAILS_END";
  try {
    await app.type("id:session-panel", `printf '\\033]777;notify;Build details;${message}\\007'`);
    await app.keyboard.press("enter");
    await app.waitFor("id:notifications-unread");
    await app.press("id:notifications");
    await app.waitFor("label:Expand Build details notification");
    expect(app.shows(`id:notification-panel >> text:${message}`)).toBe(false);
    await app.press("label:Expand Build details notification");
    await app.waitFor(`id:notification-panel >> text:${message}`);
    await app.scroll("id:notifications-history", 2000);
    await app.press("label:Collapse Build details notification");
    await app.waitFor("label:Expand Build details notification");
    expect(app.shows(`id:notification-panel >> text:${message}`)).toBe(false);
    await app.press("id:notifications-clear");
    await app.waitFor("id:notifications-empty");
  } finally { await close(app); }
});
