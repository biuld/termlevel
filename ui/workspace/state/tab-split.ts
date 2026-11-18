import { batch } from "nsh";
import { insertPane, minimum, panes, removePane, type Pane, type Size, type TilingState } from "../tiling";
import type { Tab } from "./types";

export function splitTabDrop(context: {
  tab: (id: string) => Tab | undefined;
  layoutOf: (key: string) => TilingState;
  canvasSize: () => Size;
  paneMinimum: () => Size;
  gap: () => number;
  forgetSession: (id: string) => void;
  patchTab: (id: string, patch: Partial<Tab>) => void;
  patchGroup: (id: string, patch: { collapsed: boolean }) => void;
  setLayout: (key: string, state: TilingState) => void;
  select: (id: string) => void;
  newIds: () => { pane: string; split: string };
  report: (message: string) => void;
}) {
  return (sourceId: string, targetId: string, commit = false) => {
    const source = context.tab(sourceId), target = context.tab(targetId);
    if (!source || !target || sourceId === targetId) return false;
    const key = target.groupId ?? "";
    const state = context.layoutOf(key);
    const open = panes(state.root);
    const targetPane = open.find(pane => pane.sessionId === targetId);
    if (!targetPane && open.length >= 2) {
      if (commit) context.report("Select a terminal in the existing split first.");
      return false;
    }
    const originalSource = open.find(pane => pane.sessionId === sourceId);
    let root = targetPane ? state.root : { kind: "pane" as const, id: "drop-target", sessionId: targetId };
    if (originalSource && root) root = removePane(root, originalSource.id);
    const targetNode = targetPane ?? root as Pane;
    if (!root) return false;
    const pane: Pane = originalSource ?? { kind: "pane", id: "drop-source", sessionId: sourceId };
    const preview = insertPane(root, targetNode.id, pane, "right", "drop-split");
    const required = minimum(preview, context.paneMinimum(), context.gap());
    const size = context.canvasSize();
    if (required.width > size.width || required.height > size.height) {
      if (commit) context.report("Not enough room to split these terminals.");
      return false;
    }
    if (commit) {
      const ids = context.newIds();
      const actualTarget = targetPane ?? { ...targetNode, id: ids.pane + "-target" };
      const base = targetPane ? root : actualTarget;
      const actualSource = originalSource ?? { ...pane, id: ids.pane };
      const next = insertPane(base, actualTarget.id, actualSource, "right", ids.split);
      batch(() => {
        context.forgetSession(sourceId);
        if (!targetPane) context.forgetSession(targetId);
        context.patchTab(sourceId, { groupId: target.groupId });
        if (target.groupId) context.patchGroup(target.groupId, { collapsed: false });
        context.setLayout(key, { root: next, focusedPaneId: actualSource.id, zoomed: false });
        context.select(sourceId);
        context.report("");
      });
    }
    return true;
  };
}
