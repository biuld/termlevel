import { cleanup, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { NativeInputProbe, Runtime, Sessions, SessionTerminal, type SessionTerminalRef } from "termlevel/native";

function InputFixture() {
  const session = Sessions.open(Runtime.cwd(), "#eeeeee", "#202020", "/bin/sh");
  const other = Sessions.open(Runtime.cwd(), "#eeeeee", "#202020", "/bin/sh");
  const selected = signal(true);
  const fontSize = signal(14);
  const snapshot = signal("");
  const output = signal("");
  const geometry = signal("");
  let terminal: SessionTerminalRef | undefined;
  cleanup(() => { Sessions.close(session.id); Sessions.close(other.id); });
  async function edit(action: string, text = "") {
    try { snapshot.set(JSON.stringify(await NativeInputProbe.edit(action, text))); }
    catch (error) { snapshot.set(`ERROR ${error}`); }
  }
  return <View class="flex-1 bg-[#202020]">
    <View class="min-h-0 flex-1">
      <SessionTerminal ref={ref => { terminal = ref; }} sessionId={session.id} active={selected()}
        style={{ position: "absolute", top: 0, right: 0, bottom: 0, left: 0,
          opacity: selected() ? 1 : 0, pointerEvents: selected() ? "auto" : "none" }}
        defaultForeground="#eeeeee" defaultBackground="#202020"
        fontSize={fontSize()} lineHeight={fontSize() + 8} testID="ime-terminal" class="px-(--padding-terminal) pb-(--padding-terminal)" />
      <SessionTerminal sessionId={other.id} active={!selected()}
        style={{ position: "absolute", top: 0, right: 0, bottom: 0, left: 0,
          opacity: selected() ? 0 : 1, pointerEvents: selected() ? "none" : "auto" }}
        defaultForeground="#eeeeee" defaultBackground="#202020"
        fontSize={fontSize()} lineHeight={fontSize() + 8} testID="other-terminal" class="px-(--padding-terminal) pb-(--padding-terminal)" />
    </View>
    <Text testID="probe-snapshot" class="h-5 overflow-hidden">{snapshot()}</Text>
    <Text testID="probe-geometry" class="h-5 overflow-hidden">{geometry()}</Text>
    <Text testID="probe-output" class="h-5 overflow-hidden">{JSON.stringify(output())}</Text>
    <View class="flex-row">
      <Button testID="probe-geometry-read" onPress={async () => {
        try { geometry.set(JSON.stringify(await NativeInputProbe.geometry(session.id))); }
        catch (error) { geometry.set(`ERROR ${error}`); }
      }}><Text>Geometry</Text></Button>
      <Button testID="probe-read" onPress={() => edit("read")}><Text>Read input</Text></Button>
      <Button testID="probe-mark" onPress={() => edit("mark", "zhongwen")}><Text>Mark</Text></Button>
      <Button testID="probe-convert" onPress={() => edit("mark", "中文")}><Text>Convert</Text></Button>
      <Button testID="probe-commit" onPress={() => edit("commit", "中文")}><Text>Commit</Text></Button>
      <Button testID="probe-cancel" onPress={() => edit("cancel")}><Text>Cancel</Text></Button>
      <Button testID="probe-output-read" onPress={() => output.set(NativeInputProbe.output(session.id))}><Text>Read PTY</Text></Button>
      <Button testID="probe-switch" onPress={() => selected.set(!selected())}><Text>Switch</Text></Button>
      <Button testID="probe-other-read" onPress={() => output.set(NativeInputProbe.output(other.id))}><Text>Read other</Text></Button>
      <Button testID="probe-font" onPress={() => fontSize.set(18)}><Text>Font</Text></Button>
      <Button testID="probe-focus" onPress={() => terminal?.focus()}><Text>Focus</Text></Button>
    </View>
  </View>;
}

mount(InputFixture);
