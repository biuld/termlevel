import { cleanup, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { NativeInputProbe, Runtime, Sessions, SessionTerminal } from "termlevel/native";

mount(() => {
  const session = Sessions.open(Runtime.cwd(), "#eeeeee", "#202020", "/bin/sh");
  const result = signal("");
  cleanup(() => Sessions.close(session.id));
  const pause = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
  return <View class="flex-1 bg-[#202020]">
    <SessionTerminal sessionId={session.id} testID="graphics-input-terminal"
      defaultForeground="#eeeeee" defaultBackground="#202020" fontSize={14} lineHeight={22}
      class="min-h-0 flex-1 px-(--padding-terminal) pb-(--padding-terminal)" />
    <Text>{result()}</Text>
    <Button testID="graphics-input-measure" onPress={async () => {
      const marks: number[] = [];
      const echoes: number[] = [];
      try {
        for (let word = 0; word < 8; word++) {
          for (const mark of ["z", "zhongwen", "中文"]) {
            const started = Date.now();
            const state = await NativeInputProbe.edit("mark", mark);
            if (!state.marked || state.text !== mark) throw new Error("marked text was replaced");
            marks.push(Date.now() - started);
            await pause(30);
          }
          // Exercise both ASCII and IME commits; no Enter is submitted.
          const text = word % 2 ? `english${word}x` : `中文${word}x`;
          const started = Date.now();
          await NativeInputProbe.edit("commit", text);
          while (!NativeInputProbe.output(session.id).includes(text)) {
            if (Date.now() - started > 2000) break;
            await pause(10);
          }
          echoes.push(Date.now() - started);
          // A stalled echo ends the run with the measured value, so the
          // regression surfaces by latency rather than by an outer timeout.
          if (echoes[echoes.length - 1] > 2000) break;
        }
        result.set(`GRAPHICS_INPUT_RESULT_${JSON.stringify({ marks, echoes })}`);
      } catch (error) { result.set(`GRAPHICS_INPUT_ERROR_${error}`); }
    }}><Text>Measure input</Text></Button>
  </View>;
});
