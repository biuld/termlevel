import { mount, View } from "nsh";
import { Button } from "nsh/base";
import { GitWorkspace } from "../../../ui/git/GitWorkspace";
import { createWorkspace } from "../../../ui/workspace/state";
import { parseSnapshot } from "../../../ui/workspace/workspace-snapshot";
import { previewGit } from "./preview/git";

mount(() => {
  const snapshot = parseSnapshot(JSON.stringify({ version: 1, groups: [{ id: "group", title: "Projects", cwd: "/work", color: "purple" }], tabs: [], layouts: {}, workspaceKey: "group" }));
  const workspace = createWorkspace(undefined, { snapshot });
  workspace.openGitView("changes", "group");
  let polls = 0;
  const git = { ...previewGit,
    startDiscovery: () => { polls = 0; return 1; },
    pollDiscovery: () => { polls++; return polls < 5
      ? { paths: ["/work/alpha"], done: false, error: null }
      : { paths: ["/work/alpha", "/work/beta"], done: true, error: null }; },
    cancelDiscovery: () => {},
    status: async (root: string) => ({ ...(await previewGit.status(root)), branch: root.endsWith("alpha") ? "branch-alpha" : "branch-beta" }),
  };
  return <View class="flex-1 flex-col">
    <View class="h-10 shrink-0 flex-row"><Button class="h-10 px-3" testID="show-history" onPress={() => workspace.openGitView("history", "group")}>History</Button>
    <Button class="h-10 px-3" testID="show-changes" onPress={() => workspace.openGitView("changes", "group")}>Changes</Button></View>
    <GitWorkspace workspace={workspace} git={git} preview={true} notify={() => {}} />
  </View>;
});
