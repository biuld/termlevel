import { cleanup, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Sessions } from "termlevel/native";
import { WorkspaceApp } from "../../ui/WorkspaceApp";

function WorkspaceRebindFixture() {
  const mounted = signal(true);
  // Only the test harness closes sessions on disposal. The production
  // WorkspaceApp below may unmount without ending any of its processes.
  cleanup(() => Sessions.shutdown());
  return <View class="flex-1">
    <If when={mounted()}><WorkspaceApp shell="/bin/sh" /></If>
    <Button testID="toggle-workspace" onPress={() => mounted.set(!mounted())}><Text>Toggle workspace</Text></Button>
  </View>;
}

mount(WorkspaceRebindFixture);
