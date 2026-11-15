import { mount, View } from "nsh";
import { Runtime } from "termlevel/native";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";
import { nativeGit } from "../../../ui/git/native-git";
import { createWorkspace } from "../../../ui/workspace/state";
import type { WorkspaceSnapshot } from "../../../ui/workspace/workspace-snapshot";

mount(() => {
  const snapshot: WorkspaceSnapshot = {
    version: 1,
    groups: [{ id: "group-1", title: "Termlevel", cwd: Runtime.cwd(), color: "purple", collapsed: false }],
    tabs: [],
    layouts: {},
    activeTabId: null,
    workspaceKey: "group-1",
    dashboards: ["group-1"],
    expanded: true,
    sidebarWidth: null,
    fontSize: null,
    scrollback: null,
    terminalIdentity: "termlevel",
    appearance: "dark",
    interfaceFont: null,
    monoFont: null,
    workingIndicator: "bars",
  };
  const workspace = createWorkspace(undefined, { snapshot });
  workspace.openGitView("changes", "group-1");
  return <View class="flex-1">
    <WorkspaceApp workspace={workspace} snapshot={snapshot} git={nativeGit} />
  </View>;
});
