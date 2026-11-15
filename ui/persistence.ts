import { Storage } from "nsh";
import { parseSnapshot, type WorkspaceSnapshot } from "./workspace-snapshot";

/** The app's key-value storage keeps this across launches. */
export const WORKSPACE_STORAGE_KEY = "workspace";

export async function loadWorkspaceSnapshot(): Promise<WorkspaceSnapshot | null> {
  try {
    const raw = await Storage.getItem(WORKSPACE_STORAGE_KEY);
    return raw === null ? null : parseSnapshot(raw);
  } catch {
    // A broken store must not stop the app from starting.
    return null;
  }
}

export async function writeWorkspaceSnapshot(serialized: string): Promise<void> {
  try {
    await Storage.setItem(WORKSPACE_STORAGE_KEY, serialized);
  } catch {
    // Persistence is best effort; the in-memory workspace keeps working.
  }
}
