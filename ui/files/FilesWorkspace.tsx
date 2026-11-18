import type { GitTransport } from "../git/git-types";
import { FilesPanel } from "./FilesPanel";
import type { FilesTransport } from "./types";
import type { Workspace } from "../workspace/state";

export function FilesWorkspace(props: {
  workspace: Workspace;
  files: FilesTransport;
  git?: GitTransport;
  notify: (level: "info" | "error", title: string, body: string) => void;
}) {
  const store = props.workspace;
  const groupId = () => store.workspaceKey();
  const state = () => store.files()[groupId()];
  return <FilesPanel root={store.group(groupId())?.cwd ?? ""}
    showHidden={!!state()?.showHidden}
    expanded={state()?.expanded ?? []}
    selected={state()?.selected ?? null}
    transport={props.files}
    git={props.git}
    notify={props.notify}
    onShowHidden={(value) => store.patchFiles(groupId(), { showHidden: value })}
    onExpanded={(paths) => store.patchFiles(groupId(), { expanded: paths })}
    onSelected={(path) => store.patchFiles(groupId(), { selected: path })}
    onOpenTerminal={(cwd) => store.addTab(groupId(), true, cwd)} />;
}
