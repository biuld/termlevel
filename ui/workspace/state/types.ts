import type { GroupColor } from "../../shared/theme";
import type { WorkspaceSnapshot } from "../workspace-snapshot";

/** Rust owns protocol facts, aggregation and presentation semantics. */
export type TabProgress = import("termlevel/native").ProgressInfo;
export type TabActivity = import("termlevel/native").ActivityInfo;
export type TabStatus = import("termlevel/native").TerminalStatusInfo;

export interface Tab {
  id: string;
  title: string;
  groupId: string | null;
  pinned: boolean;
  cwd: string;
  state?: string;
  exitCode?: number | null;
  error?: string | null;
  inputWarning?: string | null;
  customTitle?: boolean;
  bell?: boolean;
  progress?: TabProgress | null;
  activity?: TabActivity | null;
  status?: TabStatus;
  statusRevision?: number;
  shellPhase?: string;
  /** Latest OSC 7 / ConEmu / iTerm2 working directory. Runtime-only. */
  liveCwd?: string;
  /** OSC 22 pointer shape. Runtime-only. */
  mouseShape?: string;
  /** OSC 133 semantic-prompt state. Runtime-only. */
  atPrompt?: boolean;
}

export interface SessionTransport {
  cwd: string;
  list(): Array<{
    id: string; cwd: string; title: string; state: string; exitCode?: number | null;
    error?: string | null; progress?: TabProgress | null; activity?: TabActivity | null;
    currentCwd?: string | null; mouseShape?: string; atPrompt?: boolean;
    status?: TabStatus; statusRevision?: number; shellPhase?: string;
  }>;
  open(cwd: string): ReturnType<SessionTransport["list"]>[number];
  close(id: string): void;
}

export interface TabGroup {
  id: string;
  title: string;
  cwd: string;
  color: GroupColor;
  collapsed: boolean;
}

export interface WorkspaceOptions {
  /** The document restored from storage. Null or omitted on a first launch. */
  snapshot?: WorkspaceSnapshot | null;
}
