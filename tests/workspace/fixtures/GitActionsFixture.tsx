import { mount, View } from "nsh";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";
import { previewGit } from "./preview/git";
import { createWorkspace } from "../../../ui/workspace/state";
import { parseSnapshot } from "../../../ui/workspace/workspace-snapshot";

mount(() => {
  const snapshot = parseSnapshot(JSON.stringify({ version: 1, groups: [{ id: "termlevel", title: "termlevel", cwd: "/work", color: "purple" }], tabs: [], layouts: {}, workspaceKey: "termlevel" }));
  const workspace = createWorkspace(undefined, { snapshot });
  workspace.openGitView("history", "termlevel");
  const git = {
    ...previewGit,
    fetch: async () => { await new Promise<void>(resolve => setTimeout(resolve, 300)); return "Fetch complete."; },
    pull: async () => { throw new Error("Cannot fast-forward divergent branches."); },
  };
  return <View class="flex-1"><WorkspaceApp workspace={workspace} git={git} /></View>;
});
