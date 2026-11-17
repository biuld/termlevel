import { computed, List, signal, token, View } from "nsh";
import { Button, Input, Popover } from "nsh/base";
import { Icon } from "../shared/icons";
import { Type } from "../shared/type";
import type { Workspace } from "../workspace/state";
import type { GitRepository } from "./git-types";
import { RefHint } from "./HistoryPanel/common";

export function RepositoryPicker(props: {
  workspace: Workspace; repositories: () => GitRepository[]; selected: () => string | null;
  manual: () => boolean; scanning: () => boolean; error: () => string;
  choose: (root: string) => void; follow: () => void; add: (path: string) => void; rescan: () => void;
}) {
  const open = signal(false);
  const query = signal("");
  const path = signal("");
  const adding = signal(false);
  const current = computed(() => props.repositories().find(repo => repo.root === props.selected()));
  const filtered = computed(() => {
    const text = query().trim().toLowerCase();
    return props.repositories().filter(repo => !text || repo.label.toLowerCase().includes(text) || repo.root.toLowerCase().includes(text));
  });
  return <View class="shrink-0 flex-row items-center gap-2 border-b border-border px-3 py-1">
    <Type role="label">Repository</Type>
    <Popover open={open()} onOpenChange={({ open: value }) => { open.set(value); if (value) { query.set(""); adding.set(false); } }} placement="bottom" align="start"
      popupClass="w-(--width-repository-picker) rounded-(--radius-overlay) border border-border bg-popover p-2"
      trigger={<Button testID="git-repository-selector" accessibilityLabel="Select Git repository"
        tooltip={props.selected() ? <RefHint label={props.selected()!} /> : undefined}
        class="min-w-0 shrink flex-row items-center justify-center gap-1 rounded-(--radius-control) px-2 py-1 hover:bg-control-hover">
        <Type role="label" class="min-w-0" lines={1} overflow="ellipsis-middle">{current()?.label || (props.selected() ? "Unavailable repository" : props.scanning() ? "Searching…" : "Select repository")}</Type>
        <Icon name="down" size={12} color={token("muted-foreground")} />
      </Button>}>
      <View class="gap-1">
        <View class="h-(--height-input) flex-row items-center gap-2 rounded-(--radius-control) bg-search-surface px-2">
        <Icon name="search" size={14} color={token("muted-foreground")} />
        <Input testID="git-repository-search" accessibilityLabel="Search repositories" placeholder="Search repositories"
          value={query()} onChange={({ value }) => query.set(value)} autoFocus
          class="h-full min-w-0 flex-1 text-(length:--text-body) text-foreground" />
        </View>
        <View class="flex-row items-center justify-between px-2 pt-2 pb-1"><Type role="caption">REPOSITORIES</Type><Type role="caption">{`${filtered().length}`}</Type></View>
        <List testID="git-repositories" accessibilityLabel="Repositories" each={filtered()} keyExtractor={repo => repo.root}
          style={{ height: Math.min(288, Math.max(48, filtered().length * 48)) }} estimatedItemSize={48}
          empty={<Type role="caption" class="px-2 py-3">{props.scanning() ? "Searching for repositories…" : "No matching repositories."}</Type>}>
          {(repo) => <Button testID={`git-repository-${repo().root}`} accessibilityLabel={`Select ${repo().root}`}
            selected={repo().root === props.selected()} tooltip={<RefHint label={repo().root} />}
            onPress={() => { props.choose(repo().root); open.set(false); }}
            class={`h-12 w-full flex-col justify-center rounded-(--radius-control) px-2 hover:bg-control-hover ${repo().root === props.selected() ? "bg-control-hover" : ""}`}>
            <View class="w-full flex-row items-center gap-2">
              <Type role="body" class="min-w-0 flex-1" lines={1} overflow="ellipsis-middle">{repo().label}</Type>
              <If when={repo().kind === "linked"}><Type role="caption">worktree</Type></If>
              <If when={repo().kind === "submodule"}><Type role="caption">submodule</Type></If>
              <If when={repo().root === props.selected()}><Icon name="check" size={14} color={token("primary")} /></If>
            </View>
            <View class="w-full flex-row items-center gap-2">
              <Type role="caption" class="min-w-0 flex-1" lines={1} overflow="ellipsis-middle">{repo().root}</Type>
              <If when={props.workspace.repositorySummaries()[repo().root]}>{(summary) =>
                <Type role="caption" class="max-w-[45%]" lines={1} overflow="ellipsis-middle">{`${summary().branch} · ${summary().conflicts ? `${summary().conflicts} conflicts` : `${summary().changes} changes`}`}</Type>
              }</If>
            </View>
          </Button>}
        </List>
        <View class="flex-row items-center gap-1 border-t border-border pt-2 mt-1">
          <Button testID="git-follow-context" onPress={() => { props.follow(); open.set(false); }} class="h-7 flex-row items-center justify-center gap-1 rounded-(--radius-control) px-2 hover:bg-control-hover">
            <Icon name="terminal" size={14} color={token("muted-foreground")} /><Type role="label">{props.manual() ? "Follow terminal" : "Following terminal"}</Type>
          </Button>
          <Button testID="git-discover-refresh" onPress={props.rescan} class="ml-auto h-7 flex-row items-center justify-center gap-1 rounded-(--radius-control) px-2 hover:bg-control-hover"><Icon name="refresh" size={14} color={token("muted-foreground")} /><Type role="label">Rescan</Type></Button>
          <If when={!adding()}><Button testID="git-add-repository-open" accessibilityLabel="Add repository" tooltip="Add repository" onPress={() => adding.set(true)} class="h-7 w-7 flex-row items-center justify-center rounded-(--radius-control) hover:bg-control-hover"><Icon name="plus" size={15} color={token("muted-foreground")} /></Button></If>
        </View>
        <If when={adding()}><View class="flex-row items-center gap-2">
          <Input testID="git-add-repository-path" accessibilityLabel="Repository folder" placeholder="Add repository folder"
            value={path()} onChange={({ value }) => path.set(value)}
            class="h-(--height-input) min-w-0 flex-1 rounded-(--radius-control) border border-border px-2 text-(length:--text-body) text-foreground" />
          <Button testID="git-add-repository" disabled={!path().trim()} onPress={() => props.add(path().trim())} class="h-7 shrink-0 flex-row items-center justify-center rounded-(--radius-control) px-2 hover:bg-control-hover"><Type role="label">Add</Type></Button>
        </View></If>
        <If when={!!props.error()}><Type role="caption" tone="destructive">{props.error()}</Type></If>
      </View>
    </Popover>
    <If when={props.scanning()}><Type role="caption">Searching…</Type></If>
  </View>;
}
