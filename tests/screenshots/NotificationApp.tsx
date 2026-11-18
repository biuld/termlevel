import { cleanup, mount, signal, Text, token, View } from "nsh";
import { Notifications, Sessions } from "termlevel/native";
import { createWorkspace } from "../../ui/workspace/state";
import { WorkspaceApp } from "../../ui/app/WorkspaceApp";

// A controlled shell in a temporary directory. It never reads the user's
// workspace storage, files, Git repositories or shell profiles.
mount(() => {
  Notifications.clear();
  const workspace = createWorkspace({
    cwd: "/tmp/termlevel-screenshot-demo",
    list: Sessions.list,
    open: (cwd) => Sessions.open(cwd, token("terminal-foreground"), token("terminal-background"), "/tmp/termlevel-screenshot-shell"),
    close: Sessions.close,
  });
  const groupId = workspace.addGroup("/tmp/termlevel-screenshot-demo");
  workspace.patchGroup(groupId, { title: "Termlevel", color: "purple" });
  const ids = signal("");
  const timer = setInterval(() => ids.set(Sessions.list().map((session) => session.id).join(",")), 100);
  cleanup(() => clearInterval(timer));
  return <View class="flex-1">
    <WorkspaceApp shell="/tmp/termlevel-screenshot-shell" workspace={workspace} />
    <Text testID="session-ids" style={{ position: "absolute", left: 0, top: 0, width: 1, height: 1, opacity: 0 }}>{`ids:${ids()}`}</Text>
  </View>;
});
