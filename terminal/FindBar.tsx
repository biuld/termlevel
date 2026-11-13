import { Text, View, type Signal } from "nsh";
import { Button, Input } from "nsh/base";
import { DialogKeys, IconButton } from "../ui/controls";

/** Case-insensitive hits in terminal text, in reading order. */
export function countMatches(text: string, query: string) {
  const needle = query.toLowerCase();
  if (!needle) return 0;
  let count = 0;
  for (const line of text.split("\n")) {
    const hay = line.toLowerCase();
    let from = 0;
    while (from <= hay.length - needle.length) {
      const at = hay.indexOf(needle, from);
      if (at < 0) break;
      count += 1;
      from = at + needle.length;
    }
  }
  return count;
}

export function FindBar(props: {
  query: Signal<string>;
  count: Signal<number>;
  index: Signal<number>;
  onQuery: (value: string) => void;
  onMove: (delta: number) => void;
  onClose: () => void;
}) {
  const label = () => props.query() ? (props.count() ? `${props.index()} of ${props.count()}` : "No matches") : "";
  return <View testID="find-in-terminal" class="h-(--height-toolbar) shrink-0 flex-row items-center gap-1 border-b border-sidebar-border bg-chrome px-2">
    <DialogKeys dismiss={props.onClose} />
    <Input testID="find-query" accessibilityLabel="Find in terminal" value={props.query()} autoFocus placeholder="Find"
      onChange={({ value }) => props.onQuery(value)}
      onSubmit={() => props.onMove(1)}
      class="h-(--height-control) min-w-0 flex-1 rounded-(--radius-control) border border-sidebar-border bg-sidebar px-2 text-(length:--text-body) text-foreground" />
    <Text testID="find-count" class="shrink-0 text-(length:--text-caption) text-muted-foreground">{label()}</Text>
    <Button accessibilityLabel="Previous match" testID="find-previous" onPress={() => props.onMove(-1)}
      class="h-(--height-control) w-(--height-control) items-center justify-center rounded-(--radius-control) hover:bg-control-hover">
      <Text class="text-(length:--text-body) text-foreground">↑</Text>
    </Button>
    <Button accessibilityLabel="Next match" testID="find-next" onPress={() => props.onMove(1)}
      class="h-(--height-control) w-(--height-control) items-center justify-center rounded-(--radius-control) hover:bg-control-hover">
      <Text class="text-(length:--text-body) text-foreground">↓</Text>
    </Button>
    <IconButton icon="close" label="Close find" testID="find-close" onPress={props.onClose} />
  </View>;
}
