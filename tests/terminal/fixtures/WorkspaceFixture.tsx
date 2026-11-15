import { mount, token } from "nsh";
import { Runtime, Sessions } from "termlevel/native";
import { createWorkspace } from "../../../ui/workspace/workspace";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";

// Exercise the production workspace with a deterministic real shell. User
// shell profiles/history are covered by separate native acceptance runs.
mount(() => {
  const workspace = createWorkspace({
    cwd: Runtime.cwd(), list: Sessions.list,
    open: (cwd) => Sessions.open(cwd, token("terminal-foreground"), token("terminal-background"), "/bin/sh"),
    close: Sessions.close,
  });
  // The production app starts empty; this fixture starts with one shell.
  if (!workspace.tabs().length) workspace.addTab();
  return <WorkspaceApp shell="/bin/sh" workspace={workspace} />;
});
