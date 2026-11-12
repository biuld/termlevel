import { effect, Overlay, Text, token, View, type InputRef, type ViewRef } from "nsh";
import { Button, Input } from "nsh/base";
import { DialogKeys } from "./controls";
import { groupColor, groupPalette } from "./theme";
import type { Workspace } from "./workspace";

/** Inline group editing, anchored to the group label rather than a modal. */
export function GroupEditor(props: {
  id: string;
  anchor: ViewRef;
  workspace: Workspace;
  close: (restoreFocus?: boolean) => void;
  deleteGroup: () => void;
}) {
  const store = props.workspace;
  const id = props.id;
  let nameInput: InputRef | undefined;
  effect(() => nameInput?.focus());

  return <Overlay testID="group-editor" role="dialog" accessibilityLabel="Edit tab group"
    anchor={props.anchor} placement="right" align="start" offset={8} margin={8}
    onPressOutside={() => props.close(false)} onCancel={() => props.close(true)}
    class="w-(--width-group-editor) rounded-(--radius-overlay) border border-border bg-popover p-3"
    style={{
      boxShadow: [{ x: 0, y: 8, blur: 24, color: token("panel-shadow") }],
    }}>
    <DialogKeys dismiss={() => props.close(true)} />
    <View class="mb-2 flex-row items-center justify-between" role="group" accessibilityLabel="Group color">
      <Each of={groupPalette}>{(color) => <Button
        testID={`group-color-${color().id}`} accessibilityLabel={`${color().label} group color`}
        selected={store.group(id)?.color === color().id}
        onPress={() => { store.patchGroup(id, { color: color().id }); nameInput?.focus(); }}
        class="h-(--size-group-swatch) w-(--size-group-swatch) items-center justify-center rounded-full border-2 border-transparent focus-visible:ring-2 ring-ring"
        style={{ borderColor: store.group(id)?.color === color().id ? groupColor(color().id) : undefined }}>
        <View class="h-4 w-4 rounded-full" style={{ backgroundColor: groupColor(color().id) }} />
      </Button>}</Each>
    </View>
    <Text class="mb-2 font-mono text-(length:--text-caption) text-muted-foreground" style={{ numberOfLines: 1 }}>{store.group(id)?.cwd}</Text>
    <Input testID="group-name-input" accessibilityLabel="Group name" ref={(ref) => { nameInput = ref; }}
      value={store.group(id)?.title ?? ""} placeholder="Example: Project" maxLength={80}
      onChange={({ value }) => store.patchGroup(id, { title: value })}
      onSubmit={() => props.close(true)}
      class="mb-2 h-(--height-input) rounded-(--radius-control) border border-input bg-background px-2 text-(length:--text-body) text-foreground focus:border-ring" />
    <Button testID="group-editor-new-terminal"
      onPress={() => { props.close(false); store.addTab(id); }}
      class="h-(--height-action) justify-center rounded-(--radius-control) px-2 hover:bg-control-hover focus-visible:bg-control-hover">
      <Text class="text-(length:--text-body) text-popover-foreground">New terminal in group</Text>
    </Button>
    <Button testID="group-editor-ungroup"
      onPress={() => { props.close(false); store.ungroup(id); }}
      class="h-(--height-action) justify-center rounded-(--radius-control) px-2 hover:bg-control-hover focus-visible:bg-control-hover">
      <Text class="text-(length:--text-body) text-popover-foreground">Ungroup terminals</Text>
    </Button>
    <View class="mx-2 my-1 h-px bg-border" />
    <Button testID="group-editor-delete"
      onPress={() => { props.close(false); props.deleteGroup(); }}
      class="h-(--height-action) justify-center rounded-(--radius-control) px-2 hover:bg-control-hover focus-visible:bg-control-hover">
      <Text class="text-(length:--text-body) text-destructive">Delete group…</Text>
    </Button>
  </Overlay>;
}
