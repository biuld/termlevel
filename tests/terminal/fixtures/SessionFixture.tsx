import { cleanup, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Runtime, Sessions, SessionTerminal, type SessionInfo } from "termlevel/native";

function SessionFixture() {
  const record = Sessions.open(Runtime.cwd(), null, null, "/bin/sh");
  const info = signal<SessionInfo>(record);
  const attached = signal(true);
  const closed = signal(false);
  // The test harness owns its session. Detaching the view below does not run
  // this cleanup; disposing the complete harness explicitly closes its child.
  cleanup(() => Sessions.close(record.id));
  return <View class="flex-1">
    <If when={attached() && !closed()}>
      <SessionTerminal testID="live-terminal" sessionId={record.id}
        accessibilityLabel="Live terminal" class="min-h-0 flex-1"
        onStatusChange={({ info: next }) => info.set(next)} />
    </If>
    <Text testID="live-state">{info().state}</Text>
    <Text testID="live-exit">{info().exitCode ?? "no exit"}</Text>
    <Button testID="detach-live" onPress={() => attached.set(!attached())}><Text>Toggle attachment</Text></Button>
    <Button testID="close-live" onPress={() => {
      Sessions.close(record.id);
      closed.set(true);
    }}><Text>Close session</Text></Button>
  </View>;
}

mount(SessionFixture);
