import { cleanup, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Runtime, Sessions, SessionTerminal, NativeInputProbe } from "termlevel/native";
function Fixture() {
  const session = Sessions.open(Runtime.cwd(), "#eeeeee", "#202020", "/bin/sh");
  const frames = signal(0);
  let timer: ReturnType<typeof setInterval> | undefined;
  const stop = () => { if (timer !== undefined) clearInterval(timer); timer = undefined; };
  cleanup(() => { stop(); Sessions.close(session.id); });
  return <View class="flex-1 bg-[#202020]">
    <SessionTerminal testID="scroll-terminal" sessionId={session.id} fontFamily="Menlo"
      fontSize={13} lineHeight={21} defaultForeground="#eeeeee" defaultBackground="#202020"
      class="min-h-0 flex-1" />
    <Text>{`NATIVE_SURFACE_FRAMES_${frames()}`}</Text>
    <Text>{`SCROLL_HOST_PID_${NativeInputProbe.processId()}`}</Text>
    <Button testID="scroll-start" onPress={() => {
      stop(); void NativeInputProbe.surfaceFrames(session.id).then(n => console.log(`NATIVE_SURFACE_START_${n}`)); timer = setInterval(() => { void NativeInputProbe.wheel(session.id,21); },16);
    }}><Text>Start wheels</Text></Button>
    <Button testID="scroll-stop" onPress={() => { stop(); void NativeInputProbe.surfaceFrames(session.id).then(n => { frames.set(n); console.log(`NATIVE_SURFACE_END_${n}`); }); }}><Text>Stop wheels</Text></Button>
  </View>;
}
mount(Fixture);
