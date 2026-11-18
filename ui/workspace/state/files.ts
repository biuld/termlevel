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

export function createFiles(initial: Record<string, FilesState>) {
  const files = signal(initial);
  const write = (id: string, next: FilesState) => {
    files.update((all) => ({ ...all, [id]: next }));
  };
  const current = (id: string) => files()[id] ?? emptyFiles();
  return {
    files: files as Signal<Record<string, FilesState>>,
    show(id: string) {
      write(id, { ...current(id), open: true, showing: true });
    },
    hide(id: string) {
      const old = files()[id];
      if (!old?.showing) return;
      write(id, { ...old, showing: false });
    },
    close(id: string) {
      const old = files()[id];
      if (!old?.open && !old?.showing) return;
      write(id, { ...(old ?? emptyFiles()), open: false, showing: false });
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
