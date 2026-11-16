import { Text, token, View, type CanvasRenderingContext2D as CanvasContext } from "nsh";
import { monoFont } from "../../shared/fonts";
import type { GraphRow } from "../commit-graph";
import type { GitCommitFile, GitTransport } from "../git-types";
import type { Workspace } from "../../workspace/state";
export const LOG_PAGE = 200;
export const ROW_HEIGHT = 30;
export const LANE_GAP = 14;
export const GRAPH_LEFT = 12;

export interface CommitRow extends GraphRow {
  author: string;
  date: string;
  short: string;
  subject: string;
  active: boolean;
}

export interface FileRow extends GitCommitFile {
  active: boolean;
}

export function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message;
  return typeof error === "string" ? error : "Git request failed.";
}

export function pathName(path: string) {
  const trimmed = path.trim().replace(/[\\/]+$/, "");
  const cut = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  return (cut >= 0 ? trimmed.slice(cut + 1) : trimmed) || trimmed;
}

export function formatDate(timestamp: number) {
  const date = new Date(timestamp * 1000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function fullDate(timestamp: number) {
  const date = new Date(timestamp * 1000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function shortRef(ref: string) {
  if (ref.startsWith("HEAD -> ")) return ref.slice(8);
  if (ref.startsWith("tag: ")) return ref.slice(5);
  return ref;
}

export function refColor(ref: string) {
  if (ref.startsWith("HEAD")) return token("primary");
  if (ref.startsWith("tag: ")) return token("notification-warning");
  if (ref.includes("/")) return token("group-purple");
  return token("group-green");
}

export function statusColor(status: string) {
  switch (status) {
    case "A": return token("group-green");
    case "M": case "T": return token("notification-warning");
    case "D": case "U": return token("destructive");
    case "R": case "C": return token("primary");
    default: return token("muted-foreground");
  }
}

/** The unified-diff section for one path, or empty when the commit has no text diff for it. */
export function patchForFile(patch: string, path: string): string {
  if (!patch) return "";
  const sections: string[][] = [];
  let section: string[] | null = null;
  for (const line of patch.split("\n")) {
    if (line.startsWith("diff --git ")) {
      if (section) sections.push(section);
      section = [line];
    } else if (section) {
      section.push(line);
    }
  }
  if (section) sections.push(section);
  // A hand-written patch (or a preview fixture) may have no `diff --git` lines.
  if (!sections.length) sections.push(patch.split("\n"));
  for (const lines of sections) {
    if (lines.some((line) => line.startsWith("Binary files ") || line.startsWith("GIT binary patch"))) {
      continue;
    }
    const paths = new Set<string>();
    for (const line of lines) {
      if (line.startsWith("diff --git ")) {
        const match = /^diff --git a\/(.+?) b\/(.+)$/.exec(line);
        if (match) {
          paths.add(match[1]);
          paths.add(match[2]);
        }
      } else if (line.startsWith("+++ b/") || line.startsWith("--- a/")) {
        paths.add(line.slice(6).replace(/\t.*$/, ""));
      } else if (line.startsWith("rename to ") || line.startsWith("copy to ")) {
        paths.add(line.slice(line.indexOf("to ") + 3));
      } else if (line.startsWith("rename from ") || line.startsWith("copy from ")) {
        paths.add(line.slice(line.indexOf("from ") + 5));
      }
    }
    if (paths.has(path)) return lines.join("\n");
  }
  return "";
}

export function laneColor(lane: number) {
  const colors = [
    token("group-green"),
    token("group-blue"),
    token("group-purple"),
    token("group-orange"),
    token("group-pink"),
    token("group-cyan"),
    token("group-red"),
    token("group-yellow"),
  ];
  return colors[lane % colors.length];
}

export function drawGraph(cx: CanvasContext, size: { width: number; height: number }, row: GraphRow) {
  const mid = size.height / 2;
  const x = (lane: number) => GRAPH_LEFT + lane * LANE_GAP;
  const line = (lane: number, from: number, to: number, color: string) => {
    cx.beginPath();
    cx.moveTo(x(lane), from);
    cx.lineTo(x(lane), to);
    cx.strokeStyle = color;
    cx.stroke();
  };
  cx.lineWidth = 1.5;
  cx.lineCap = "round";
  cx.lineJoin = "round";
  for (const lane of row.through) line(lane, 0, size.height, laneColor(lane));
  for (let lane = 0; lane < row.before.length; lane++) {
    if (lane === row.lane) {
      line(lane, 0, mid, laneColor(lane));
    } else if (!row.after.includes(row.before[lane])) {
      line(lane, 0, mid, laneColor(lane));
    }
  }
  for (const edge of row.edges) {
    if (edge.to === row.lane) {
      line(edge.to, mid, size.height, laneColor(edge.to));
      continue;
    }
    cx.beginPath();
    cx.moveTo(x(row.lane), mid);
    cx.bezierCurveTo(
      x(row.lane), mid + (size.height - mid) * 0.35,
      x(edge.to), size.height - (size.height - mid) * 0.35,
      x(edge.to), size.height,
    );
    cx.strokeStyle = laneColor(edge.to);
    cx.stroke();
  }
  cx.beginPath();
  cx.arc(x(row.lane), mid, 3.5, 0, Math.PI * 2);
  cx.fillStyle = laneColor(row.lane);
  cx.fill();
}

export function RefChip(props: { label: string }) {
  const color = () => refColor(props.label);
  return <View class="shrink-0 rounded-(--radius-tab) border px-1" style={{ borderColor: color() }}>
    <Text style={{
      fontFamily: monoFont(),
      fontSize: token("text-caption"),
      lineHeight: token("leading-caption"),
      color: color(),
      numberOfLines: 1,
    }}>{shortRef(props.label)}</Text>
  </View>;
}

/** Branches, worktrees, commit lanes and the selected commit's patch for one group. */

export type HistoryProps = { root?: () => string; refreshTick?: () => number; rescan?: () => void; workspace: Workspace; git: GitTransport; preview: boolean; notify: (level: "info" | "error", title: string, body: string) => void };
