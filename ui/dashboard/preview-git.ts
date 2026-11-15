import type { GitTransport } from "./git-types";

/** Deterministic preview: a clean repository, never starts git or reads storage. */
export const previewGit: GitTransport = {
  status: async (root) => ({
    repo: true,
    root,
    branch: "main",
    detached: false,
    upstream: null,
    ahead: 0,
    behind: 0,
    operation: null,
    changes: [],
    truncated: false,
    error: null,
  }),
  diff: async () => "",
};
