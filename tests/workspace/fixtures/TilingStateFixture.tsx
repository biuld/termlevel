import { mount, Text, View } from "nsh";
import { createWorkspace } from "../../../ui/workspace/workspace";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";

mount(() => {
  const workspace = createWorkspace();
  return <View class="flex-1">
    <WorkspaceApp workspace={workspace} />
    <Text testID="tiling-state" class="h-4" style={{ numberOfLines: 1 }}>{JSON.stringify({ tiling: workspace.tiling(), activeId: workspace.activeId() })}</Text>
  </View>;
});
