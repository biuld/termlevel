import { Git } from "termlevel/native";
import type { GitTransport } from "./git-types";

/** Real Git data from the system CLI through the Rust read-only module. */
export const nativeGit: GitTransport = {
  status: (root) => Git.status(root),
  diff: (root, path, mode) => Git.diff(root, path, mode),
};
