import type { GitCommit } from "./git-types";

export interface GraphEdge { from: number; to: number }
export interface GraphGeometry {
  incoming: boolean;
  lane: number;
  through: number[];
  edges: GraphEdge[];
}
export interface GraphRow extends GraphGeometry { commit: GitCommit }
export interface GraphLayout { rows: GraphRow[]; maxLanes: number; clipped: boolean }

/** Deterministic fallback for injected/preview transports. Production lays out pages in Rust. */
export function layoutCommits(commits: GitCommit[]): GraphLayout {
  const targets = new Map<string, number>();
  const slots: Array<string | null> = [];
  const free = new Set<number>();
  let maxLanes = 0;
  let clipped = false;
  const insert = (oid: string, preferred?: number) => {
    const found = targets.get(oid);
    if (found !== undefined) return found;
    const lane = preferred !== undefined && free.has(preferred) ? preferred : free.values().next().value ?? slots.length;
    free.delete(lane);
    slots[lane] = oid;
    targets.set(oid, lane);
    maxLanes = Math.max(maxLanes, Math.min(lane + 1, 25));
    clipped ||= lane >= 24;
    return lane;
  };
  const rows = commits.map(commit => {
    const incoming = targets.has(commit.oid);
    const lane = insert(commit.oid);
    const through: number[] = [];
    for (let i = 0; i < Math.min(slots.length, 24); i++) {
      if (i !== lane && slots[i] !== null) through.push(i);
    }
    targets.delete(commit.oid);
    slots[lane] = null;
    free.add(lane);
    const edges = commit.parents.map((parent, index) => ({ from: Math.min(lane, 24), to: Math.min(insert(parent, index === 0 ? lane : undefined), 24) }));
    return { commit, incoming, lane: Math.min(lane, 24), through, edges };
  });
  return { rows, maxLanes, clipped };
}
