import { expect,open,test } from "nsh/test";
test("native AppKit Kitty press repeat release reach the real PTY",async()=>{
  const app=await open("./terminal/fixtures/ProtocolsFixture.tsx",{platform:"macos",width:900,height:600});
  try {
    await app.wait(200);
    await app.type("id:protocol-terminal","python3 -u terminal/fixtures/protocol_probe.py");
    await app.keyboard.press("enter");
    await app.waitFor("text:KITTY_READY");
    await app.press("id:native-key-sequence");
    await app.waitFor("text:KITTY_EVENTS_OK");
    await app.screenshot("artifacts/native-kitty-events.png");
  }finally {app.dispose();}
});

test("native PNG rendering, OSC8 and terminal selection copy, OSC52",async()=>{
  const app=await open("./terminal/fixtures/ProtocolsFixture.tsx",{platform:"macos",width:900,height:600});
  try {
    await app.wait(200);await app.press("id:clipboard-save");
    await app.type("id:protocol-terminal","python3 terminal/fixtures/graphics_probe.py");await app.keyboard.press("enter");
    await app.waitFor("text:GRAPHICS_READY");
    await app.press("id:clipboard-system-read");await app.waitFor("text:CLIP_TERMLEVEL_OSC52");
    await app.screenshot("artifacts/native-terminal-graphics.png");
    await app.drag("text:LINKED_WORD",{x:50});
    await app.keyboard.press("mod+c");await app.press("id:clipboard-read");await app.wait(100);
    const copied=app.texts().find(t=>t.startsWith("CLIP_"))!;expect(copied!=="CLIP_TERMLEVEL_OSC52"&&copied.length>5).toBe(true);
    await app.keyboard.press("mod+a");await app.keyboard.press("mod+c");await app.press("id:clipboard-read");
    await app.waitUntil(()=>app.texts().some(t=>t.startsWith("CLIP_")&&t.includes("LINKED_WORD")&&t.includes("GRAPHICS_READY")));
    await app.screenshot("artifacts/native-terminal-selection.png");
  } finally {await app.press("id:clipboard-restore");app.dispose();}
});

test("native pointer reports SGR clicks and OSC52 reads require the setting",async()=>{
  const app=await open("./terminal/fixtures/ProtocolsFixture.tsx",{platform:"macos",width:900,height:600});
  async function command(value:string){await app.type("id:protocol-terminal",value);await app.keyboard.press("enter");}
  try {
    await app.wait(200);await app.press("id:clipboard-save");
    await command("printf '\\033]52;c;VEVSTUxFVkVMX09TQzUy\\007'");
    await command("python3 terminal/fixtures/clipboard_probe.py deny");await app.waitFor("text:OSC52_READ_DENY_OK");
    await app.press("id:clipboard-allow");
    await command("python3 terminal/fixtures/clipboard_probe.py allow");await app.waitFor("text:OSC52_READ_ALLOW_OK");
    await command("python3 terminal/fixtures/mouse_probe.py");await app.waitFor("text:MOUSE_READY");
    await app.press("id:protocol-terminal");await app.waitFor("text:MOUSE_EVENTS_OK");
  } finally {await app.press("id:clipboard-restore");app.dispose();}
});
