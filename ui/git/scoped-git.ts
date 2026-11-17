import { cleanup } from "nsh";
import type { GitTransport } from "./git-types";

/** A mounted view owns its reads; mutations keep their repository-wide lifetime. */
export function scopedGit(transport: GitTransport): GitTransport {
  const scoped = transport.fork?.() ?? transport;
  if (scoped !== transport) cleanup(() => scoped.dispose?.());
  return scoped;
}
