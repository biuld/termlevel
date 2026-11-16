import { expect, test } from "nsh/test";
import { createGitSchedule } from "../../ui/git/scheduler";
import { gitOperation, gitOperationBusy } from "../../ui/git/operation-gate";

test("Git task deadlines pause and do not replay missed intervals", () => {
  const calls: string[] = [];
  const tick = createGitSchedule({ scan: () => calls.push("scan"), refresh: () => calls.push("refresh"), fetch: () => calls.push("fetch") }, 0);
  tick(5_000, true);
  tick(300_000, false);
  expect(calls.join(",")).toBe("refresh");
  tick(300_000, true);
  tick(300_001, true);
  expect(calls.join(",")).toBe("refresh,scan,refresh,fetch");
});

test("repository operations remain exclusive across callers and release on failure", async () => {
  let release: () => void = () => {};
  const running = gitOperation("/repo", () => new Promise<void>(resolve => { release = resolve; }));
  expect(gitOperationBusy("/repo")).toBe(true);
  let rejected = false;
  try { await gitOperation("/repo", async () => {}); } catch { rejected = true; }
  expect(rejected).toBe(true);
  release();
  await running;
  try { await gitOperation("/repo", async () => { throw new Error("failure"); }); } catch {}
  expect(gitOperationBusy("/repo")).toBe(false);
});
