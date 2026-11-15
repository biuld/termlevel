import { cleanup, mount, signal, Text, token, View } from "nsh";
import { Button } from "nsh/base";
import { Runtime, Sessions } from "termlevel/native";
import { createWorkspace, type Workspace } from "../../../ui/workspace/workspace";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";

function WorkspaceRebindFixture(props: { workspace: Workspace }) {
  const mounted = signal(true);
  // Only the test harness closes sessions on disposal. The production
  // WorkspaceApp below may unmount without ending any of its processes.
  cleanup(() => Sessions.shutdown());
  return <View class="flex-1">
    <If when={mounted()}><WorkspaceApp workspace={props.workspace} shell="/bin/sh" /></If>
    <Button testID="toggle-workspace" onPress={() => mounted.set(!mounted())}><Text>Toggle workspace</Text></Button>
  </View>;
}

mount(() => {
  const workspace = createWorkspace({
    cwd: Runtime.cwd(), list: Sessions.list,
    open: (cwd) => Sessions.open(cwd, token("terminal-foreground"), token("terminal-background"), "/bin/sh"),
    close: Sessions.close,
  });
  if (!workspace.tabs().length) workspace.addTab();
  return <WorkspaceRebindFixture workspace={workspace} />;
});
