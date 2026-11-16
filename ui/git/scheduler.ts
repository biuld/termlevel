import { cleanup, lifecycle } from "nsh";

type Tasks = { scan: () => void; refresh: () => void; fetch: () => void };

/** Advances independent deadlines without replaying missed ticks after suspension. */
export function createGitSchedule(tasks: Tasks, start: number) {
  let nextScan = start + 30_000;
  let nextRefresh = start + 5_000;
  let nextFetch = start + 300_000;
  return (now: number, active: boolean) => {
    if (!active) return;
    if (now >= nextScan) { nextScan = now + 30_000; tasks.scan(); }
    if (now >= nextRefresh) { nextRefresh = now + 5_000; tasks.refresh(); }
    if (now >= nextFetch) { nextFetch = now + 300_000; tasks.fetch(); }
  };
}

/** A single clock for background work while the Git workspace is mounted. */
export function scheduleGitTasks(tasks: Tasks) {
  const tick = createGitSchedule(tasks, Date.now());
  const timer = setInterval(() => tick(Date.now(), lifecycle() === "active"), 1000);
  cleanup(() => clearInterval(timer));
}
