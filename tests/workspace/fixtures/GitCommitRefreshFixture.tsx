import { mount, View } from "nsh";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";
import { previewGit } from "./preview/git";
import { createWorkspace } from "../../../ui/workspace/state";
import { parseSnapshot } from "../../../ui/workspace/workspace-snapshot";

mount(() => {
  const snapshot = parseSnapshot(JSON.stringify({ version: 1, groups: [{ id: "termlevel", title: "termlevel", cwd: "/work", color: "purple" }], tabs: [], layouts: {}, workspaceKey: "termlevel" }));
  const workspace = createWorkspace(undefined, { snapshot });
  workspace.openGitView("history", "termlevel");
  let action = "";
  const git = {
    ...previewGit,
    fetch: async () => { action = "fetch"; return "Fetched."; },
    pull: async () => { action = "pull"; return "Pulled."; },
    push: async () => { action = "push"; return "Pushed."; },
    log: async (root: string, scope: string, limit: number, skip: number) => {
      const page = await previewGit.log(root, scope, limit, skip);
      if (!action) return page;
      const commit = { ...page.commits[0], oid: action.padEnd(40, "0"), short: action, subject: `New commit after ${action}`, refs: [] };
      return { ...page, commits: [commit, ...page.commits] };
    },
  };
  return <View class="flex-1"><WorkspaceApp workspace={workspace} git={git} /></View>;
});
