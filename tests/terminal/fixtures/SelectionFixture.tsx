import { cleanup, mount, signal, Text, View } from "nsh";
import { NativeInputProbe, Runtime, Sessions, SessionTerminal } from "termlevel/native";

mount(() => {
  const session = Sessions.open(Runtime.cwd(), null, null, "/bin/sh");
  const summary = signal("{}");
  const read = () => {
    const text = NativeInputProbe.selection(session.id) ?? "";
    const matches = [...text.matchAll(/SELECT_(\d+)/g)].map(match => Number(match[1]));
    summary.set(JSON.stringify({ cols: NativeInputProbe.columns(session.id), lines: text ? text.split("\n").length : 0, first: matches[0] ?? -1, last: matches[matches.length - 1] ?? -1 }));
  };
  const timer = setInterval(read, 80);
  cleanup(() => { clearInterval(timer); Sessions.close(session.id); });
  return <View class="flex-1 bg-chrome">
    <SessionTerminal testID="selection-terminal" sessionId={session.id} fontFamily="Menlo" fontSize={13} lineHeight={20} class="min-h-0 flex-1 px-(--padding-terminal) pb-(--padding-terminal)" />
    <Text class="h-4">{`SELECTION_SESSION_${session.id}`}</Text>
    <Text testID="selection-summary" class="h-4">{summary()}</Text>
  </View>;
});
