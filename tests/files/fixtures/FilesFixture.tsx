import { mount, signal, View } from "nsh";
import { Button } from "nsh/base";
import { FilesWorkspace } from "../../../ui/files/FilesWorkspace";
import type { FileText, FilesTransport } from "../../../ui/files/types";
import { Type } from "../../../ui/shared/type";
import { createWorkspace } from "../../../ui/workspace/state";

const text = new Map<string, string>([
  ["readme.md", "hello"],
  ["notes.txt", "notes"],
]);

function fileText(path: string, value: string): FileText {
  return { text: value, binary: false, truncated: false, lossy: false, bytes: value.length, modifiedMs: 1 };
}

mount(() => {
  const workspace = createWorkspace();
  const calls = signal<string[]>([]);
  workspace.openFiles("termlevel");
  const transport: FilesTransport = {
    list: async (_root, directory) => directory === ""
      ? { entries: [
        { name: "notes.txt", kind: "file", hidden: false },
        { name: "readme.md", kind: "file", hidden: false },
      ], truncated: false }
      : { entries: [], truncated: false },
    read: async (_root, path) => fileText(path, text.get(path) ?? ""),
    save: async (_root, path, value) => {
      text.set(path, value);
      calls.update((all) => [...all, `save:${path}:${value}`]);
      return fileText(path, value);
    },
    absolute: async (_root, path) => `/proj/${path}`,
    terminalDirectory: async (_root, path) => path.includes("/") ? `/proj/${path.slice(0, path.lastIndexOf("/"))}` : "/proj",
    reveal: async (_root, path) => { calls.update((all) => [...all, `reveal:${path}`]); },
    openWithDefaultApp: async (_root, path) => { calls.update((all) => [...all, `open:${path}`]); },
    trash: async (_root, path) => { calls.update((all) => [...all, `trash:${path}`]); },
  };
  return <View testID="files-fixture" class="absolute inset-0 flex-col">
    <View class="h-10 shrink-0 flex-row items-center gap-2 px-2">
      <Button testID="close-files" class="h-8 px-2" onPress={() => workspace.closeFiles("termlevel")}>Close files</Button>
      <Button testID="show-terminal" class="h-8 px-2" onPress={() => workspace.select("prototype")}>Show terminal</Button>
      <Type testID="file-calls" role="caption">{calls().join(" ")}</Type>
    </View>
    <View class="min-h-0 flex-1">
      <If when={workspace.filesShowing()}>
        <FilesWorkspace workspace={workspace} files={transport} notify={() => {}} />
      </If>
    </View>
  </View>;
});
