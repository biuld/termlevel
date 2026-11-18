import { Text, token, View } from "nsh";
import { Button } from "nsh/base";
import { Hint, IconButton } from "../shared/controls";
import { Icon } from "../shared/icons";
import { groupColor } from "../shared/theme";
import type { GitView } from "./workspace-snapshot";
import type { Workspace } from "./state";

export function visibleGitTabs(store: Workspace, id: string, expanded: boolean): GitView[] {
  return (store.gitViews()[id]?.open ?? []).filter((kind) =>
    !store.group(id)?.collapsed || (!expanded && store.workspaceKey() === id && store.activeGitView() === kind));
}

export function filesTabVisible(store: Workspace, id: string, expanded: boolean): boolean {
  if (!store.files()[id]?.open) return false;
  return !store.group(id)?.collapsed || (!expanded && store.workspaceKey() === id && store.filesShowing());
}

export function hiddenToolCount(store: Workspace, id: string, expanded: boolean): number {
  const git = (store.gitViews()[id]?.open.length ?? 0) - visibleGitTabs(store, id, expanded).length;
  const files = (store.files()[id]?.open ? 1 : 0) - (filesTabVisible(store, id, expanded) ? 1 : 0);
  return git + files;
}

export function ToolTabs(props: { store: Workspace; id: string; expanded: boolean }) {
  const store = props.store;
  const id = props.id;
  const git = () => visibleGitTabs(store, id, props.expanded);
  const files = () => filesTabVisible(store, id, props.expanded);
  const label = (kind: GitView) => kind === "changes" ? "Changes" : "History";
  return <>
    <Each of={git()}>{(kind) =>
      <View class="group relative my-0.5 h-(--height-tab) flex-row items-center rounded-(--radius-tab) hover:bg-tab-hover"
        classList={{ "mx-1": !props.expanded, "bg-tab-selected hover:bg-tab-selected": store.workspaceKey() === id && store.activeGitView() === kind() }}>
        <Button testID={`git-tab-${id}-${kind()}`} role="tab" selected={store.workspaceKey() === id && store.activeGitView() === kind()}
          accessibilityLabel={`${label(kind())} in ${store.groupTitle(id)}`}
          onPress={() => store.openGitView(kind(), id)}
          tooltip={<Hint>{`${label(kind())} · ${store.groupTitle(id)}`}</Hint>}
          class="h-full min-w-0 flex-1 flex-row items-center gap-2 rounded-(--radius-tab) focus-visible:ring-2 ring-sidebar-ring"
          style={{ justifyContent: props.expanded ? "flex-start" : "center", paddingHorizontal: props.expanded ? 7 : 0 }}>
          <Icon name={kind() === "changes" ? "gitBranch" : "gitCommit"} size={13} color={groupColor(store.group(id)?.color)} />
          <If when={props.expanded}><Text class="min-w-0 flex-1 text-(length:--text-body) text-sidebar-foreground">{label(kind())}</Text></If>
        </Button>
        <If when={props.expanded}><View class="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
          style={{ opacity: store.workspaceKey() === id && store.activeGitView() === kind() ? 1 : undefined }}>
          <IconButton icon="close" label={`Close ${label(kind())}`} testID={`close-git-${id}-${kind()}`} onPress={() => store.closeGitView(id, kind())} />
        </View></If>
      </View>
    }</Each>
    <If when={files()}>
      <View class="group relative my-0.5 h-(--height-tab) flex-row items-center rounded-(--radius-tab) hover:bg-tab-hover"
        classList={{ "mx-1": !props.expanded, "bg-tab-selected hover:bg-tab-selected": store.workspaceKey() === id && store.filesShowing() }}>
        <Button testID={`files-tab-${id}`} role="tab" selected={store.workspaceKey() === id && store.filesShowing()}
          accessibilityLabel={`Files in ${store.groupTitle(id)}`}
          onPress={() => store.openFiles(id)}
          tooltip={<Hint>{`Files · ${store.groupTitle(id)}`}</Hint>}
          class="h-full min-w-0 flex-1 flex-row items-center gap-2 rounded-(--radius-tab) focus-visible:ring-2 ring-sidebar-ring"
          style={{ justifyContent: props.expanded ? "flex-start" : "center", paddingHorizontal: props.expanded ? 7 : 0 }}>
          <Icon name="files" size={13} color={groupColor(store.group(id)?.color)} />
          <If when={props.expanded}><Text class="min-w-0 flex-1 text-(length:--text-body) text-sidebar-foreground">Files</Text></If>
        </Button>
        <If when={props.expanded}><View class="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
          style={{ opacity: store.workspaceKey() === id && store.filesShowing() ? 1 : undefined }}>
          <IconButton icon="close" label="Close Files" testID={`close-files-${id}`} onPress={() => store.closeFiles(id)} />
        </View></If>
      </View>
    </If>
  </>;
}
