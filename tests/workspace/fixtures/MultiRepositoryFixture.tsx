import { mount, signal, View } from "nsh";
import { Button } from "nsh/base";
import { GitWorkspace } from "../../../ui/git/GitWorkspace";
import type { GitCommit, GitRepository, GitTransport } from "../../../ui/git/git-types";
import { createWorkspace } from "../../../ui/workspace/state";
import { parseSnapshot } from "../../../ui/workspace/workspace-snapshot";
import { Type } from "../../../ui/shared/type";
import { previewGit } from "./preview/git";

const alpha = "/work/alpha";
const beta = "/work/beta";
const gamma = "/other/gamma";
const repo = (root: string, id: number): GitRepository => ({ id, root, label: root.split("/").pop()!, gitDir: `${root}/.git`, commonDir: `${root}/.git`, kind: "main" });
mount(() => {
  const snapshot = parseSnapshot(JSON.stringify({ version: 1, groups: [
    { id: "one", title: "Work", cwd: "/work", color: "purple" },
    { id: "two", title: "Other", cwd: "/other", color: "blue" },
    { id: "many", title: "Many", cwd: "/many", color: "blue" },
  ], tabs: [], layouts: {}, workspaceKey: "one" }));
  let sessionIndex = 0;
  const workspace = createWorkspace({ cwd: "/work", list: () => [],
    open: cwd => ({ id: `mock-${++sessionIndex}`, cwd, title: "Terminal", state: "running", currentCwd: cwd === "/work" ? `${alpha}/src` : `${gamma}/src` }), close: () => {},
  }, { snapshot });
  workspace.addTab("one");
  const terminal = workspace.tabs()[0].id;
  workspace.addTab("two", false);
  workspace.openGitView("history", "one");
  const calls = signal<Record<string, number>>({});
  const resumes = signal(0);
  const removed = signal(false);
  const visible = signal(true);
  const revision = signal(1);
  let next = 0;
  const subscriptions = new Map<number, { root: string; contexts: string[] }>();
  const inspect = (root: string) => root.startsWith(alpha) ? alpha : root.startsWith(beta) ? beta : root.startsWith(gamma) ? gamma : null;
  function transport(): GitTransport {
    const cursor = new Map<string, number>();
    return { ...previewGit, fork: transport, cancel: () => {}, dispose: () => {},
      startDiscovery: (root, contexts = []) => { const id = ++next; subscriptions.set(id, { root, contexts }); return id; },
      updateDiscovery: (id, contexts) => { subscriptions.get(id)!.contexts = contexts; revision.update(value => value + 1); },
      refreshDiscovery: () => revision.update(value => value + 1),
      cancelDiscovery: id => { subscriptions.delete(id); },
      pollDiscovery: (id, _revision, offset = 0) => {
        const sub = subscriptions.get(id)!;
        const entries = sub.root === "/work" ? [repo(alpha, 1), ...removed() ? [] : [repo(beta, 2)]]
          : sub.root === "/many" ? Array.from({ length: 1500 }, (_, index) => repo(`/many/project-${index}`, index + 10)) : [repo(gamma, 3)];
        const roots = sub.contexts.flatMap(path => { const root = inspect(path); return root ? [{ path, root }] : []; });
        const page = entries.slice(offset, offset + 256);
        return { repositories: page, paths: page.map(repo => repo.root), revision: revision(), total: entries.length,
          done: true, scanning: false, preferredRoot: roots[0]?.root ?? null, contextRoots: roots };
      },
      status: async root => ({ ...await previewGit.status(root), root, branch: root.split("/").pop() }),
      branches: async root => ({ revision: "fixed", branches: [{ name: "main", fullName: "refs/heads/main", kind: "local", current: true, target: `${root}-0` }] }),
      log: async (root, _scope, limit, skip) => {
        calls.update(all => ({ ...all, [root]: (all[root] ?? 0) + 1 }));
        const commits: GitCommit[] = Array.from({ length: Math.min(limit, 600 - skip) }, (_, index) => {
          const number = skip + index;
          return { oid: `${root}-${number}`, short: String(number), parents: [`${root}-${number + 1}`],
            subject: `${root.split("/").pop()} commit ${number}`, author: "Test", email: "test@example.com", timestamp: 1700000000, refs: [] };
        });
        cursor.set(root, skip + commits.length);
        return { commits, hasMore: skip + commits.length < 600, truncated: false };
      },
      resumeHistory: async (root, _scope, count) => { resumes.update(value => value + 1); return cursor.get(root) === count; },
      commit: async (root, oid) => ({ ...await previewGit.commit(root, oid), commit: {
        oid, short: oid.split("-").pop()!, parents: [], subject: `${root.split("/").pop()} commit ${oid.split("-").pop()}`, author: "Test", email: "test@example.com", timestamp: 1700000000, refs: [],
      } }),
    };
  }
  const git = transport();
  return <View class="relative flex-1 flex-col" onLayout={({ bounds }) => workspace.setCanvasSize(bounds.size)}>
    <View class="h-8 shrink-0 flex-row gap-1">
      <Button class="h-8 px-2" testID="group-one" onPress={() => workspace.openGitView("history", "one")}><Type role="label">Work</Type></Button>
      <Button class="h-8 px-2" testID="group-two" onPress={() => workspace.openGitView("history", "two")}><Type role="label">Other</Type></Button>
      <Button class="h-8 px-2" testID="group-many" onPress={() => workspace.openGitView("history", "many")}><Type role="label">Many</Type></Button>
      <Button class="h-8 px-2" testID="cd-beta" onPress={() => workspace.patchTab(terminal, { liveCwd: `${beta}/src` })}><Type role="label">cd beta</Type></Button>
      <Button class="h-8 px-2" testID="hide-git" onPress={() => visible.set(!visible())}><Type role="label">Toggle Git</Type></Button>
      <Button class="h-8 px-2" testID="remove-beta" onPress={() => { removed.set(!removed()); revision.update(value => value + 1); }}><Type role="label">Toggle beta</Type></Button>
      <Type role="caption">{`Alpha logs ${calls()[alpha] ?? 0} · Beta logs ${calls()[beta] ?? 0} · Resumes ${resumes()}`}</Type>
    </View>
    <If when={visible()}><GitWorkspace workspace={workspace} git={git} preview={true} notify={() => {}} /></If>
  </View>;
});
