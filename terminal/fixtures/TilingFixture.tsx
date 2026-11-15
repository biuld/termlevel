import { mount, signal, Text, token, View } from "nsh";
import { Button } from "nsh/base";
import { NativeInputProbe, Runtime, Sessions } from "termlevel/native";
import { createWorkspace } from "../../ui/workspace";
import { WorkspaceApp } from "../../ui/WorkspaceApp";

mount(() => {
  const workspace = createWorkspace({ cwd: Runtime.cwd(), list: Sessions.list,
    open: cwd => Sessions.open(cwd, token("terminal-foreground"), token("terminal-background"), "/bin/sh"), close: Sessions.close });
  // A real fixture starts with one shell; the app itself starts empty.
  if (!workspace.tabs().length) workspace.addTab();
  const snapshot = signal("");
  const probe = signal("");
  const output = signal("");
  const geometry = signal("");
  async function edit(action: string, text = "") {
    try { probe.set(JSON.stringify(await NativeInputProbe.edit(action, text))); }
    catch (error) { probe.set(`ERROR ${error}`); }
  }
  return <View class="flex-1 bg-chrome text-foreground">
    <WorkspaceApp workspace={workspace} />
    <View class="h-8 flex-row items-center gap-2">
      <Button testID="read-sessions" onPress={() => snapshot.set(JSON.stringify(Sessions.list()))}><Text>Read sessions</Text></Button>
      <Text testID="session-snapshot" class="min-w-0 flex-1 text-(length:--text-caption)" style={{ numberOfLines: 1 }}>{snapshot()}</Text>
    </View>
    <View class="h-8 flex-row items-center gap-2">
      <Button testID="tile-probe-swap" onPress={() => workspace.focusDirection("right", true)}><Text>Swap focused</Text></Button>
      <Button testID="tile-probe-read" onPress={() => edit("read")}><Text>Read input</Text></Button>
      <Button testID="tile-probe-mark" onPress={() => edit("mark", "zhongwen")}><Text>Mark</Text></Button>
      <Button testID="tile-probe-convert" onPress={() => edit("mark", "中文")}><Text>Convert</Text></Button>
      <Button testID="tile-probe-commit" onPress={() => edit("commit", "中文")}><Text>Commit</Text></Button>
      <Button testID="tile-probe-cancel" onPress={() => edit("cancel")}><Text>Cancel</Text></Button>
      <Button testID="tile-probe-output" onPress={() => output.set(JSON.stringify(Sessions.list().map(s => ({ id: s.id, output: NativeInputProbe.output(s.id) }))))}><Text>Read outputs</Text></Button>
      <Button testID="tile-probe-geometry" onPress={async () => {
        try { geometry.set(JSON.stringify(await Promise.all(Sessions.list().map(async s => ({ id: s.id, ...await NativeInputProbe.geometry(s.id) }))))); }
        catch (error) { geometry.set(`ERROR ${error}`); }
      }}><Text>Geometry</Text></Button>
    </View>
    <Text testID="tile-probe-snapshot" class="h-4 text-(length:--text-caption)" style={{ numberOfLines: 1 }}>{probe()}</Text>
    <Text testID="tile-probe-output-text" class="h-4 text-(length:--text-caption)" style={{ numberOfLines: 1 }}>{output()}</Text>
    <Text testID="tile-probe-geometry-text" class="h-4 text-(length:--text-caption)" style={{ numberOfLines: 1 }}>{geometry()}</Text>
  </View>;
});
