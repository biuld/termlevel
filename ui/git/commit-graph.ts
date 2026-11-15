import type { GitCommit } from "./git-types";

/** A parent line leaving the commit's dot for another lane. */
export interface GraphEdge {
  from: number;
  to: number;
}

/** One commit row with the lanes that enter and leave it. */
export interface GraphRow {
  commit: GitCommit;
  /** The lane holding this commit's dot. */
  lane: number;
  /** The commits each lane expects at the top of the row. */
  before: string[];
  /** The commits each lane expects at the bottom of the row. */
  after: string[];
  /** Lines from the dot to each parent at the bottom. */
  edges: GraphEdge[];
  /** Lanes that continue straight through this row. */
  through: number[];
}

export interface GraphLayout {
  rows: GraphRow[];
  maxLanes: number;
}

/**
 * Assigns stable lanes to a newest-first, topologically ordered log.
 *
 * A commit keeps the lane it was expected in; its first parent continues that
 * lane, and each further parent starts or joins another one. Lanes that expect
 * the same commit collapse into the first, so two branches that meet do not
 * draw two parallel lines for the same history.
 */
export function layoutCommits(commits: GitCommit[]): GraphLayout {
  let lanes: string[] = [];
  const rows: GraphRow[] = [];
  for (const commit of commits) {
    let index = lanes.indexOf(commit.oid);
    if (index < 0) {
      index = lanes.length;
      lanes.push(commit.oid);
    }
    const before = [...lanes];
    const after = [...lanes];
    after.splice(index, 1);
    const edges: GraphEdge[] = [];
    for (let parentIndex = 0; parentIndex < commit.parents.length; parentIndex++) {
      const parent = commit.parents[parentIndex];
      let lane = after.indexOf(parent);
      if (lane < 0) {
        lane = parentIndex === 0 ? Math.min(index, after.length) : after.length;
        after.splice(lane, 0, parent);
      }
      edges.push({ from: index, to: lane });
    }
    // Two lanes expecting the same commit collapse into the first.
    for (let lane = after.length - 1; lane >= 0; lane--) {
      const first = after.indexOf(after[lane]);
      if (first === lane) continue;
      for (const edge of edges) {
        if (edge.to === lane) edge.to = first;
      }
      after.splice(lane, 1);
      for (const edge of edges) {
        if (edge.to > lane) edge.to -= 1;
      }
    }
    const through: number[] = [];
    for (let lane = 0; lane < before.length; lane++) {
      if (lane !== index && after.includes(before[lane])) through.push(lane);
    }
    rows.push({ commit, lane: index, before, after, edges, through });
    lanes = after;
  }
  const maxLanes = rows.reduce(
    (max, row) => Math.max(max, row.before.length, row.after.length, row.lane + 1),
    0,
  );
  return { rows, maxLanes };
}
