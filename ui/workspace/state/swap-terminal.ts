import { batch, type Signal } from "nsh";
import { panes, updateNode, type TilingState } from "../tiling";
import type { Tab } from "./types";

export function swapTerminal(
  tabs: Signal<Tab[]>,
  layouts: Signal<Record<string, TilingState>>,
  workspaceKey: Signal<string>,
  select: (id: string) => void,
) {
  return (currentId: string, replacementId: string) => {
    const current = tabs().find(tab => tab.id === currentId);
    const replacement = tabs().find(tab => tab.id === replacementId);
    const activeLayout = layouts()[workspaceKey()];
    const currentPane = panes(activeLayout?.root ?? null).find(pane => pane.sessionId === currentId);
    if (!current || !replacement || !currentPane || panes(activeLayout.root).some(pane => pane.sessionId === replacementId)) return;

    batch(() => {
      tabs.update(all => all.map(tab => {
        if (tab.id === currentId) return { ...tab, groupId: replacement.groupId };
        if (tab.id === replacementId) return { ...tab, groupId: current.groupId };
        return tab;
      }));
      layouts.update(all => Object.fromEntries(Object.entries(all).map(([key, state]) => {
        let root = state.root;
        for (const pane of panes(state.root)) {
          if (pane.sessionId === currentId || pane.sessionId === replacementId) {
            root = updateNode(root!, pane.id, () => ({
              ...pane, sessionId: pane.sessionId === currentId ? replacementId : currentId,
            }));
          }
        }
        return [key, { ...state, root, focusedPaneId: key === workspaceKey() ? currentPane.id : state.focusedPaneId }];
      })));
      select(replacementId);
    });
  };
}
