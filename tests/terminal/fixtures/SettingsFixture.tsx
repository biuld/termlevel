import { mount, token } from "nsh";
import { Runtime, Sessions } from "termlevel/native";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";
import { createWorkspace } from "../../../ui/workspace/state";

mount(() => {
  const workspace = createWorkspace({
    cwd: Runtime.cwd(),
    list: Sessions.list,
    open: (cwd) => Sessions.open(cwd, token("terminal-foreground"), token("terminal-background"), "/bin/sh"),
    close: Sessions.close,
  });
  if (!workspace.tabs().length) workspace.addTab();
  return <WorkspaceApp workspace={workspace} />;
});
