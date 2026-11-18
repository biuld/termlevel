import { cleanup, mount, signal, Text, token, View } from "nsh";
import { Notifications, Sessions } from "termlevel/native";
import { createWorkspace } from "../../ui/workspace/state";
import { WorkspaceApp } from "../../ui/app/WorkspaceApp";

// Hero fixture: an ungrouped split workspace plus a tab group whose three
// sessions report idle agent identities through OSC 7501. The shell is a
// temporary wrapper in /tmp; no user workspace, Git, file or profile is read.
mount(() => {
  Notifications.clear();
  const workspace = createWorkspace({
    cwd: "/tmp/termlevel-screenshot-demo",
    list: Sessions.list,
    open: (cwd) => Sessions.open(cwd, token("terminal-foreground"), token("terminal-background"), "/tmp/termlevel-screenshot-shell"),
    close: Sessions.close,
  });
  const ungrouped = workspace.addTab(null, true, "/tmp/termlevel-screenshot-demo");
  const groupId = workspace.addGroup("/tmp/termlevel-screenshot-demo");
  workspace.patchGroup(groupId, { title: "Agents", color: "purple" });
  const agent1 = workspace.tabs().find((tab) => tab.groupId === groupId)!.id;
  const agent2 = workspace.addTab(groupId, true, "/tmp/termlevel-screenshot-demo");
  const agent3 = workspace.addTab(groupId, true, "/tmp/termlevel-screenshot-demo");
  workspace.select(ungrouped);
  const ids = signal("");
  const timer = setInterval(() => ids.set(Sessions.list().map((session) => session.id).join(",")), 100);
  cleanup(() => clearInterval(timer));
  return <View class="flex-1">
    <WorkspaceApp shell="/tmp/termlevel-screenshot-shell" workspace={workspace} />
    <Text testID="session-ids" style={{ position: "absolute", left: 0, top: 0, width: 1, height: 1, opacity: 0 }}>{`ids:${ids()}`}</Text>
    <Text testID="screenshot-ids" style={{ position: "absolute", left: 0, top: 0, width: 1, height: 1, opacity: 0 }}>{`u:${ungrouped};a:${agent1},${agent2},${agent3}`}</Text>
  </View>;
});
