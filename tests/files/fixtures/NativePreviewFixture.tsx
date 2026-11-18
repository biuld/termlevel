import { mount, signal, View } from "nsh";
import { FilesPanel } from "../../../ui/files/FilesPanel";
import { nativeFileTransport } from "../../../ui/files/native-files";
import type { FilesLeave } from "../../../ui/workspace/state/files";

mount(() => {
  const selected = signal<string | null>(null);
  const expanded = signal<string[]>([]);
  return <View class="flex-1">
    <FilesPanel root="/tmp/termlevel-file-preview-test" selected={selected()} expanded={expanded()} showHidden={false}
      transport={nativeFileTransport} onShowHidden={() => {}} onExpanded={expanded.set} onSelected={selected.set}
      onOpenTerminal={() => {}} notify={() => {}} filesUnsaved={signal(false)}
      filesLeave={signal<FilesLeave | null>(null)} releaseFiles={() => {}} />
  </View>;
});
