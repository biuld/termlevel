import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Sessions } from "termlevel/native";
import { loadWorkspaceSnapshot } from "../../../ui/workspace/persistence";
import type { WorkspaceSnapshot } from "../../../ui/workspace/workspace-snapshot";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";

// Keep production session creation in WorkspaceApp; expose IDs only for targeting.
mount(() => {
  const ids = signal("[]");
  const loaded = signal<{ snapshot: WorkspaceSnapshot | null } | null>(null);
  void loadWorkspaceSnapshot().then(snapshot => loaded.set({ snapshot }));
  return <View class="flex-1">
    <If when={loaded()}>{(value) => <WorkspaceApp shell="/bin/sh" snapshot={value().snapshot} />}</If>
    <Button testID="read-identities" onPress={() => ids.set(JSON.stringify(Sessions.list().map(session => session.id)))}><Text>Read session IDs</Text></Button>
    <Text testID="identity-sessions">{ids()}</Text>
  </View>;
});
