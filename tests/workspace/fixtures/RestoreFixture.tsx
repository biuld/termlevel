import { mount, signal, View } from "nsh";
import { createWorkspace } from "../../../ui/workspace/workspace";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";
import { loadWorkspaceSnapshot } from "../../../ui/workspace/persistence";
import type { WorkspaceSnapshot } from "../../../ui/workspace/workspace-snapshot";

// The fake transport gives restored tabs deterministic new session ids without
// starting processes. Storage is the headless test's own seeded key-value store.
let sequence = 0;
const transport = {
  cwd: "/Users/test",
  list: () => [],
  open: (cwd: string) => {
    const id = `session-${++sequence}`;
    // One sentinel lets a test exercise the missing-directory fallback.
    return cwd === "/missing"
      ? { id, cwd, title: "", state: "error", exitCode: null, error: "Cannot open directory" }
      : { id, cwd, title: `Shell ${sequence}`, state: "running", exitCode: null, error: null };
  },
  close: () => {},
};

function RestoredWorkspace(props: { snapshot: WorkspaceSnapshot | null }) {
  const workspace = createWorkspace(transport, { snapshot: props.snapshot });
  return <WorkspaceApp workspace={workspace} snapshot={props.snapshot} persist />;
}

mount(() => {
  const loaded = signal<{ snapshot: WorkspaceSnapshot | null } | null>(null);
  void loadWorkspaceSnapshot().then((snapshot) => { if (!loaded()) loaded.set({ snapshot }); });
  return <View class="flex-1">
    <If when={loaded()}>{(value) => <RestoredWorkspace snapshot={value().snapshot} />}</If>
  </View>;
});
