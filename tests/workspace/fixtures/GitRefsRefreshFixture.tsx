import { mount, signal, View } from "nsh";
import { Button } from "nsh/base";
import { HistoryPanel } from "../../../ui/git/HistoryPanel";
import { previewGit } from "./preview/git";
import { createWorkspace } from "../../../ui/workspace/state";
import { parseSnapshot } from "../../../ui/workspace/workspace-snapshot";

mount(() => {
  const snapshot = parseSnapshot(JSON.stringify({ version: 1, groups: [{ id: "group", title: "Project", cwd: "/work", color: "purple" }], tabs: [], layouts: {}, workspaceKey: "group" }));
  const workspace = createWorkspace(undefined, { snapshot });
  const tick = signal(0);
  let pushed = false;
  let detached = false;
  const git = { ...previewGit,
    status: async (root: string) => ({ ...await previewGit.status(root), detached, branch: detached ? "9f3c1a2" : "main" }),
    branches: async (root: string) => {
      const page = await previewGit.log(root, "", 100, 0);
      return { branches: [
        { name: "main", fullName: "refs/heads/main", kind: "local" as const, current: !detached, target: page.commits[0].oid },
        { name: "origin/main", fullName: "refs/remotes/origin/main", kind: "remote" as const, current: false, target: page.commits[pushed ? 0 : 3].oid },
      ], error: null };
    },
    push: async () => { pushed = true; return "Pushed."; },
    log: async (root: string, scope: string, limit: number, skip: number) => {
      const page = await previewGit.log(root, scope, limit, skip);
      return { ...page, commits: page.commits.map((commit, index) => ({ ...commit,
        refs: index === 0 ? [...(detached ? ["HEAD", "main"] : ["HEAD -> main"]), ...(pushed ? ["origin/main"] : [])]
          : index === 3 && !pushed ? ["origin/main"] : [],
      })) };
    },
  };
  return <View class="flex-1 flex-col">
    <Button testID="detach-head" class="h-8 px-2" onPress={() => { detached = true; tick.update(value => value + 1); }}>Detach HEAD without moving refs</Button>
    <HistoryPanel workspace={workspace} git={git} preview={false} refreshTick={tick} notify={() => {}} />
  </View>;
});
