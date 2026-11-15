import { PreviewTerminal } from "./preview/Terminal";
import { previewGit } from "./preview/git";
import { mount, Text, View } from "nsh";
import { createWorkspace } from "../../../ui/workspace/state";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";

mount(() => {
  const workspace = createWorkspace();
  return <View class="flex-1">
    <WorkspaceApp workspace={workspace} git={previewGit} previewSurface={PreviewTerminal} />
    <Text testID="tiling-state" class="h-4" style={{ numberOfLines: 1 }}>{JSON.stringify({ tiling: workspace.tiling(), activeId: workspace.activeId() })}</Text>
  </View>;
});
