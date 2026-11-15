import { expect, test } from "nsh/test";
import { layoutCommits } from "../../ui/dashboard/commit-graph";
import type { GitCommit } from "../../ui/dashboard/git-types";

function commit(oid: string, parents: string[], refs: string[] = []): GitCommit {
  return {
    oid,
    short: oid,
    parents,
    author: "Test",
    email: "test@example.com",
    timestamp: 0,
    refs,
    subject: oid,
  };
}

test("a linear history stays in one lane", () => {
  const layout = layoutCommits([
    commit("c", ["b"]),
    commit("b", ["a"]),
    commit("a", []),
  ]);
  expect(layout.maxLanes).toBe(1);
  expect(layout.rows.map((row) => row.lane)).toEqual([0, 0, 0]);
  expect(layout.rows[1].through).toEqual([]);
  expect(layout.rows[2].after).toEqual([]);
});

test("a merge keeps the first parent in the commit's lane", () => {
  const layout = layoutCommits([
    commit("m", ["b", "c"], ["HEAD -> main"]),
    commit("c", ["a"]),
    commit("b", ["a"]),
    commit("a", []),
  ]);
  expect(layout.maxLanes).toBe(2);
  expect(layout.rows[0].lane).toBe(0);
  expect(layout.rows[0].edges).toEqual([
    { from: 0, to: 0 },
    { from: 0, to: 1 },
  ]);
  expect(layout.rows[0].after).toEqual(["b", "c"]);
  expect(layout.rows[1].lane).toBe(1);
  expect(layout.rows[2].lane).toBe(0);
  // Both children meet at the same parent; the lanes collapse into one.
  expect(layout.rows[3].before).toHaveLength(1);
  expect(layout.rows[3].before[0]).toBe("a");
});
