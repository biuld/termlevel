import { signal, token, View } from "nsh";
import { Button, Input, Popover } from "nsh/base";
import { Menu } from "nsh/desktop";
import { DialogKeys, Hint, IconButton } from "../shared/controls";
import { Icon } from "../shared/icons";
import { groupColor, groupPalette } from "../shared/theme";
import { Type } from "../shared/type";
import type { Workspace } from "../workspace/workspace";
import { GitPanel } from "./GitPanel";
import { HistoryPanel } from "./HistoryPanel";
import type { GitTransport } from "./git-types";

/** A group's full-canvas overview. Sessions stay mounted behind it. */
export function Dashboard(props: {
  workspace: Workspace;
  preview: boolean;
  closeGroup: (id: string) => void;
  git: GitTransport;
}) {
  const store = props.workspace;
  const groupId = () => store.workspaceKey();
  const group = () => store.group(groupId());
  const colorOpen = signal(false);
  const view = signal<"changes" | "history">("changes");

  function groupMenu() {
    void Menu.popup([
      { label: "Ungroup terminals", onSelect: () => store.ungroup(groupId()) },
      { type: "separator" },
      { label: "Delete group…", onSelect: () => props.closeGroup(groupId()) },
    ]);
  }

  return <View testID="dashboard" class="absolute inset-0 overflow-hidden rounded-(--radius-panel) bg-terminal-background"
    style={{ zIndex: 30 }}>
    <DialogKeys dismiss={store.closeDashboard} />

    {/* The group's settings live in the overview header, not a separate card. */}
    <View class="shrink-0 border-b border-border">
      <View class="h-[56px] flex-row items-center gap-3 px-3">
        <Popover open={colorOpen()} onOpenChange={({ open }) => colorOpen.set(open)}
          placement="bottom" align="start" offset={8}
          popupClass="rounded-(--radius-overlay) border border-border bg-popover p-2"
          popupStyle={{ boxShadow: [{ x: 0, y: 8, blur: 24, color: token("panel-shadow") }] }}
          trigger={<Button testID="group-color-trigger" accessibilityLabel="Change group color"
            tooltip={<Hint>Change group color</Hint>}
            class="h-7 w-7 items-center justify-center rounded-full hover:bg-control-hover focus-visible:ring-2 ring-ring">
            <View style={{
              width: token("size-group-marker"), height: token("size-group-marker"),
              borderRadius: token("size-group-marker") / 2,
              borderBottomLeftRadius: token("radius-group-corner"),
              backgroundColor: groupColor(group()?.color),
            }} />
          </Button>}>
          <View class="gap-2">
            <Type role="label">Group color</Type>
            <View class="w-[84px] flex-row flex-wrap gap-1.5">
              <Each of={groupPalette}>{(color) => <Button
                testID={`group-color-${color().id}`} accessibilityLabel={`${color().label} group color`}
                selected={group()?.color === color().id}
                onPress={() => { store.patchGroup(groupId(), { color: color().id }); colorOpen.set(false); }}
                class="h-(--size-group-swatch) w-(--size-group-swatch) items-center justify-center rounded-full border-2 border-transparent focus-visible:ring-2 ring-ring"
                style={{ borderColor: group()?.color === color().id ? groupColor(color().id) : undefined }}>
                <View class="h-3.5 w-3.5 rounded-full" style={{ backgroundColor: groupColor(color().id) }} />
              </Button>}</Each>
            </View>
          </View>
        </Popover>
        <View class="min-w-0 flex-1 flex-col justify-center gap-0.5">
          <Input testID="group-name-input" accessibilityLabel="Group name"
            value={group()?.title ?? ""} placeholder="Example: Project" maxLength={80}
            onChange={({ value }) => { if (group()) store.patchGroup(groupId(), { title: value }); }}
            class="h-(--height-tab) w-[220px] rounded-(--radius-control) bg-transparent px-1 text-(length:--text-title) font-semibold hover:bg-control-hover focus:bg-control-hover" />
          <Type role="body" family="mono" tone="muted-foreground" class="min-w-0 px-1" lines={1}>{group()?.cwd ?? ""}</Type>
        </View>
        <View class="flex-row rounded-(--radius-tab) bg-chrome p-0.5">
          <Button testID="dashboard-view-changes" selected={view() === "changes"}
            onPress={() => view.set("changes")}
            class="h-(--height-control) items-center justify-center rounded-(--radius-control) px-2.5 focus-visible:ring-2 ring-sidebar-ring"
            classList={{ "bg-sidebar": view() === "changes" }}>
            <Type role="label" tone={view() === "changes" ? "foreground" : "muted-foreground"}>Changes</Type>
          </Button>
          <Button testID="dashboard-view-history" selected={view() === "history"}
            onPress={() => view.set("history")}
            class="h-(--height-control) items-center justify-center rounded-(--radius-control) px-2.5 focus-visible:ring-2 ring-sidebar-ring"
            classList={{ "bg-sidebar": view() === "history" }}>
            <Type role="label" tone={view() === "history" ? "foreground" : "muted-foreground"}>History</Type>
          </Button>
        </View>
        <View class="flex-row items-center gap-1">
          <Button testID="dashboard-new-terminal" onPress={() => store.addTab(groupId())}
            class="h-(--height-action) flex-row items-center gap-1.5 rounded-(--radius-control) border border-sidebar-border px-2.5 hover:bg-control-hover active:bg-control-pressed focus-visible:ring-2 ring-ring">
            <Icon name="plus" size={12} color={token("icon")} />
            <Type role="body">New terminal</Type>
          </Button>
          <IconButton icon="more" label="Group actions" testID="dashboard-group-menu" onPress={groupMenu} />
          <IconButton icon="close" label="Close overview" testID="dashboard-close" onPress={store.closeDashboard} />
        </View>
      </View>
    </View>

    <If when={view() === "changes"}>
      <GitPanel workspace={store} git={props.git} preview={props.preview} />
    </If>
    <If when={view() === "history"}>
      <HistoryPanel workspace={store} git={props.git} preview={props.preview} />
    </If>
  </View>;
}
