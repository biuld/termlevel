import { expect, test } from "nsh/test";
import { layoutCommits } from "../../ui/git/commit-graph";
import type { GitCommit } from "../../ui/git/git-types";

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
  expect(layout.rows[2].edges).toEqual([]);
  expect(layout.rows.map(row => row.incoming)).toEqual([false, true, true]);
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
  expect(layout.rows[0].incoming).toBe(false);
  expect(layout.rows[1].lane).toBe(1);
  expect(layout.rows[2].lane).toBe(0);
  // A shared ancestor keeps its slot: the through line and parent edge meet it.
  expect(layout.rows[2].through).toEqual([1]);
  expect(layout.rows[2].edges).toEqual([{ from: 0, to: 1 }]);
  expect(layout.rows[3].lane).toBe(1);
  expect(layout.rows[3].through).toEqual([]);
});


test("unrelated tips have no incoming line and wide graph payloads stay bounded", () => {
  const layout = layoutCommits(Array.from({ length: 1000 }, (_, index) => commit(`tip-${index}`, [`parent-${index}`])));
  expect(layout.clipped).toBe(true);
  expect(layout.maxLanes).toBe(25);
  expect(layout.rows.every(row => !row.incoming && row.through.length <= 24)).toBe(true);
});
