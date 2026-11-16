/** Serializes repository mutations across Git view remounts. */
const busy = new Set<string>();

export function gitOperationBusy(root: string) { return busy.has(root); }

export async function gitOperation<T>(root: string, run: () => Promise<T>): Promise<T> {
  if (busy.has(root)) throw new Error("Another Git operation is in progress for this repository.");
  busy.add(root);
  try { return await run(); } finally { busy.delete(root); }
}
