import { mount, token } from "nsh";
import { Notifications, Runtime, Sessions } from "termlevel/native";
import { createWorkspace } from "../../../ui/workspace/state";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";

// Real shell with production persistence, for OSC 7 live-directory tests.
mount(() => {
  Notifications.clear();
  const workspace = createWorkspace({
    cwd: Runtime.cwd(), list: Sessions.list,
    open: (cwd) => Sessions.open(cwd, token("terminal-foreground"), token("terminal-background"), "/bin/sh"),
    close: Sessions.close,
  });
  if (!workspace.tabs().length) workspace.addTab();
  return <WorkspaceApp shell="/bin/sh" workspace={workspace} persist />;
});
