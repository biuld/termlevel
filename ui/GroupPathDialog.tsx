import { Text, View } from "nsh";
import { Button } from "nsh/base";
import { Action, AppDialog, DialogActions } from "./controls";
import { monoFont } from "./fonts";
import { Type } from "./type";

/** Picks the folder a new group starts in. The group does not exist until a path is chosen. */
export function GroupPathDialog(props: {
  open: boolean;
  paths: string[];
  error: string;
  dismiss: () => void;
  onChoose: (cwd: string) => void;
  onBrowse: () => void;
}) {
  return <AppDialog testID="new-group-path" open={props.open} label="New group" title="New group"
    frame="w-[420px]" dismiss={props.dismiss} closeLabel="Cancel new group">
    <Type role="caption">Choose the folder this group starts in. A terminal opens there after you choose.</Type>
    <View class="gap-0.5">
      <Each of={props.paths}>{(cwd, index) =>
        <Button testID={`group-path-${index()}`} accessibilityLabel={cwd()} onPress={() => props.onChoose(cwd())}
          class="h-(--height-tab) items-start justify-center rounded-(--radius-tab) px-2 hover:bg-tab-hover focus-visible:ring-2 ring-sidebar-ring">
          <Text class="text-(length:--text-body) text-foreground" style={{ fontFamily: monoFont(), numberOfLines: 1 }}>{cwd()}</Text>
        </Button>
      }</Each>
    </View>
    <If when={!!props.error}>
      <Type role="caption">{props.error}</Type>
    </If>
    <DialogActions>
      <Action onPress={props.dismiss}>Cancel</Action>
      <Action testID="group-path-browse" onPress={props.onBrowse}>Choose another folder…</Action>
    </DialogActions>
  </AppDialog>;
}
