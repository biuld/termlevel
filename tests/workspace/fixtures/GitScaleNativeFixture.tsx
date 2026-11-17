import { cleanup, mount, signal, View } from "nsh";
import { Button } from "nsh/base";
import { Runtime } from "termlevel/native";
import { HistoryPanel } from "../../../ui/git/HistoryPanel";
import { nativeGit } from "../../../ui/git/native-git";
import type { GitTransport } from "../../../ui/git/git-types";
import { Type } from "../../../ui/shared/type";
import { createWorkspace } from "../../../ui/workspace/state";
import { parseSnapshot } from "../../../ui/workspace/workspace-snapshot";

mount(() => {
  const root = `${Runtime.cwd()}/artifacts/git-scale-repo`;
  const snapshot = parseSnapshot(JSON.stringify({ version: 1, groups: [{ id: "group", title: "Large repository", cwd: root, color: "purple" }], tabs: [], layouts: {}, workspaceKey: "group" }));
  const workspace = createWorkspace(undefined, { snapshot });
  const tick = signal(0);
  const refCount = signal(0);
  const unchanged = signal(0);
  const logCount = signal(0);
  const heartbeat = signal(0);
  const maxGap = signal(0);
  let last = Date.now();
  const timer = setInterval(() => {
    const now = Date.now();
    maxGap.set(Math.max(maxGap(), now - last));
    last = now;
    heartbeat.update(value => value + 1);
  }, 20);
  cleanup(() => clearInterval(timer));
  const observe = (transport: GitTransport): GitTransport => ({ ...transport,
    fork: () => observe(transport.fork!()),
    branches: async root => {
      const refs = await transport.branches(root);
      if (refs.unchanged) unchanged.update(value => value + 1);
      else refCount.set(refs.branches.length);
      return refs;
    },
    log: async (root, scope, limit, skip) => {
      const result = await transport.log(root, scope, limit, skip);
      logCount.update(value => value + 1);
      return result;
    },
  });
  return <View class="relative flex-1 flex-col" onLayout={({ bounds }) => workspace.setCanvasSize(bounds.size)}>
    <View class="h-8 shrink-0 flex-row items-center gap-3 px-3">
      <Button class="h-8 shrink-0 px-3" testID="scale-native-refresh" onPress={() => tick.update(value => value + 1)}>Refresh refs</Button>
      <Button class="h-8 shrink-0 px-3" testID="scale-native-reset" onPress={() => { last = Date.now(); maxGap.set(0); }}>Reset measurement</Button>
      <Type testID="scale-native-stats" role="caption">{`Refs ${refCount()} · Unchanged ${unchanged()} · Logs ${logCount()} · Ticks ${heartbeat()} · Max gap ${maxGap()} ms`}</Type>
    </View>
    <HistoryPanel workspace={workspace} root={() => root} git={observe(nativeGit)} preview={false} refreshTick={tick} notify={() => {}} />
  </View>;
});
