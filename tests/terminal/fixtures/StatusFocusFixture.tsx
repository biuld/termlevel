import { cleanup, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Runtime, Sessions } from "termlevel/native";
import { createWorkspace } from "../../../ui/workspace/state";
import { TerminalSurface } from "../../../ui/terminal/TerminalSurface";

// Rebinding an active production surface must not acknowledge unseen results.
mount(() => {
  const workspace = createWorkspace({
    cwd: Runtime.cwd(), list: Sessions.list,
    open: cwd => Sessions.open(cwd, null, null, "/bin/sh"), close: Sessions.close,
  });
  const id = workspace.addTab();
  cleanup(() => Sessions.close(id));
  const attached = signal(true);
  const finding = signal(false);
  const query = signal("");
  const passiveFocus = signal(0);
  const select = workspace.select;
  workspace.select = (id, acknowledge = true) => {
    if (!acknowledge) passiveFocus.update(n => n + 1);
    select(id, acknowledge);
  };
  return <View class="flex-1">
    <View class="relative min-h-0 flex-1">
      <If when={attached()}>
        <TerminalSurface id={id} workspace={workspace} preview={false} finding={finding} findQuery={query} />
      </If>
    </View>
    <Text accessibilityLabel={`activity:${workspace.tab(id)?.activity?.state ?? "none"}:${workspace.tab(id)?.activity?.unread ? "unread" : "read"}`} />
    <Text accessibilityLabel={`passive-focus:${passiveFocus()}`} />
    <Button testID="close-status-session" onPress={() => { attached.set(false); Sessions.close(id); }}><Text>Close session</Text></Button>
    <Button testID="toggle-attachment" onPress={() => attached.set(!attached())}><Text>Toggle attachment</Text></Button>
  </View>;
});
