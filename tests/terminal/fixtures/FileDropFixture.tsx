import { cleanup, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Runtime, Sessions, SessionTerminal } from "termlevel/native";

function Fixture() {
  const sessions = [0, 1].map(() => Sessions.open(Runtime.cwd(), null, null, "/bin/sh"));
  const active = signal(0);
  const blocked = signal(false);
  cleanup(() => sessions.forEach(session => Sessions.close(session.id)));
  return <View class="flex-1">
    <View class="flex-1 flex-row">
      {sessions.map((session, index) => <SessionTerminal testID={`drop-terminal-${index}`} sessionId={session.id}
        active={active() === index && !blocked()} inputBlocked={blocked()}
        onFocusRequest={() => active.set(index)} class="flex-1" />)}
    </View>
    <Text>{`ACTIVE_${active()}`}</Text>
    <Button testID="block-drop" onPress={() => blocked.set(!blocked())}><Text>Toggle input block</Text></Button>
  </View>;
}
mount(Fixture);
