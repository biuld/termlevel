import { signal, type Signal } from "nsh";

/** Persisted file view for one group. Listings and unsaved text stay in memory. */
export interface FilesState {
  open: boolean;
  showing: boolean;
  showHidden: boolean;
  expanded: string[];
  selected: string | null;
}

export function emptyFiles(): FilesState {
  return { open: false, showing: false, showHidden: false, expanded: [], selected: null };
}

/** A leave that waits until unsaved edits are saved or discarded. */
export type FilesLeave = { id: string; action: "hide" | "close" };

export function createFiles(initial: Record<string, FilesState>) {
  const files = signal(initial);
  const unsaved = signal(false);
  const leave = signal<FilesLeave | null>(null);
  const write = (id: string, next: FilesState) => {
    files.update((all) => ({ ...all, [id]: next }));
  };
  const current = (id: string) => files()[id] ?? emptyFiles();
  const blocked = (id: string, action: FilesLeave["action"]) => {
    const old = files()[id];
    if (!unsaved() || !old?.showing) return false;
    leave.set({ id, action });
    return true;
  };
  const applyHide = (id: string) => {
    const old = files()[id];
    if (!old?.showing) return;
    write(id, { ...old, showing: false });
  };
  const applyClose = (id: string) => {
    const old = files()[id];
    if (!old?.open && !old?.showing) return;
    write(id, { ...(old ?? emptyFiles()), open: false, showing: false });
  };
  return {
    files: files as Signal<Record<string, FilesState>>,
    unsaved,
    leave,
    /** Completes a blocked hide or close after the edits are saved or discarded. */
    release() {
      const request = leave();
      unsaved.set(false);
      leave.set(null);
      if (!request) return;
      if (request.action === "close") applyClose(request.id);
      else applyHide(request.id);
    },
    show(id: string) {
      write(id, { ...current(id), open: true, showing: true });
    },
    hide(id: string) {
      if (blocked(id, "hide")) return;
      applyHide(id);
    },
    close(id: string) {
      if (blocked(id, "close")) return;
      applyClose(id);
    },
    patch(id: string, patch: Partial<FilesState>) {
      write(id, { ...current(id), ...patch });
    },
    forget(id: string) {
      files.update((all) => {
        if (!(id in all)) return all;
        const copy = { ...all };
        delete copy[id];
        return copy;
      });
    },
  };
}
