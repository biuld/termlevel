import { mount, token, View, Text } from "nsh";
import { Notifications, Runtime, Sessions } from "termlevel/native";
import { createWorkspace } from "../../../ui/workspace/state";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";

// A real /bin/sh session exercises OSC 9;4 and OSC 7501 through the same
// parser and sidebar path as production. Preview remains a separate mode.
mount(() => {
  Notifications.clear();
  const workspace = createWorkspace({
    cwd: Runtime.cwd(), list: Sessions.list,
    open: (cwd) => Sessions.open(cwd, token("terminal-foreground"), token("terminal-background"), "/bin/sh"),
    close: Sessions.close,
  });
  if (!workspace.tabs().length) workspace.addTab();
  return <View class="flex-1">
    <Text testID="program-read" accessibilityLabel={`activity:${workspace.tab(workspace.activeId() ?? "")?.activity?.state ?? "none"}:${workspace.tab(workspace.activeId() ?? "")?.activity?.unread ? "unread" : "read"}`} />
    <WorkspaceApp shell="/bin/sh" workspace={workspace} />
  </View>;
});
