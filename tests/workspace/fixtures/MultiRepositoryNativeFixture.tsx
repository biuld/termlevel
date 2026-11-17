import { mount, signal, View } from "nsh";
import { Button } from "nsh/base";
import { Runtime } from "termlevel/native";
import { GitWorkspace } from "../../../ui/git/GitWorkspace";
import { nativeGit } from "../../../ui/git/native-git";
import type { GitTransport } from "../../../ui/git/git-types";
import { Type } from "../../../ui/shared/type";
import { createWorkspace } from "../../../ui/workspace/state";
import { parseSnapshot } from "../../../ui/workspace/workspace-snapshot";
mount(() => {
  const root = `${Runtime.cwd()}/artifacts/multi-repository-repos`;
  const snapshot = parseSnapshot(JSON.stringify({ version: 1, groups: [
    { id: "container", title: "Projects", cwd: root, color: "purple" },
    { id: "inside", title: "Inside", cwd: `${root}/alpha/src`, color: "blue" },
  ], tabs: [], layouts: {}, workspaceKey: "container" }));
  const workspace = createWorkspace({ cwd: root, list: () => [], open: cwd => ({ id: "fake", cwd, title: "Terminal", state: "running" }), close: () => {} }, { snapshot });
  workspace.openGitView("history", "container");
  workspace.openGitView("history", "inside");
  workspace.openGitView("history", "container");
  const logs = signal(0);
  const resumes = signal(0);
  const visible = signal(true);
  const observe = (git: GitTransport): GitTransport => ({ ...git, fork: () => observe(git.fork!()),
    log: async (...args) => { const page = await git.log(...args); logs.update(value => value + 1); return page; },
    resumeHistory: async (...args) => { const result = await git.resumeHistory!(...args); if (result) resumes.update(value => value + 1); return result; },
  });
  return <View class="flex-1" onLayout={({ bounds }) => workspace.setCanvasSize(bounds.size)}>
    <View class="h-8 shrink-0 flex-row gap-3">
      <Button class="h-8 shrink-0 px-3" testID="repo-inside" onPress={() => workspace.openGitView("history", "inside")}><Type role="label">Inside</Type></Button>
      <Button class="h-8 shrink-0 px-3" testID="repo-container" onPress={() => workspace.openGitView("history", "container")}><Type role="label">Projects</Type></Button>
      <Button class="h-8 shrink-0 px-3" testID="repo-hide" onPress={() => visible.set(!visible())}><Type role="label">Toggle Git</Type></Button>
      <Type role="caption">{`Logs ${logs()} · Resumes ${resumes()}`}</Type>
    </View>
    <If when={visible()}><GitWorkspace workspace={workspace} git={observe(nativeGit)} preview={false} notify={() => {}} /></If>
  </View>;
});
