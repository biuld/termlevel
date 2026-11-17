import type { TreeItem } from "nsh/base";
import type { GitChange } from "./git-types";

interface TreeNode {
  name: string;
  path: string;
  children: Map<string, TreeNode>;
  change?: GitChange;
}

interface ChangeTree {
  items: TreeItem[];
  folders: string[];
  files: string[];
  changes: Map<string, GitChange>;
}

/** Groups changed paths into a directory tree. */
export function buildTree(changes: GitChange[]): ChangeTree {
  const root = new Map<string, TreeNode>();
  for (const change of changes) {
    const parts = change.path.split("/");
    let level = root;
    let path = "";
    for (let index = 0; index < parts.length; index++) {
      const name = parts[index];
      path = path ? `${path}/${name}` : name;
      let node = level.get(name);
      if (!node) {
        node = { name, path, children: new Map() };
        level.set(name, node);
      }
      if (index === parts.length - 1) node.change = change;
      level = node.children;
    }
  }
  const folders: string[] = [];
  const files: string[] = [];
  const byId = new Map<string, GitChange>();
  const toItems = (level: Map<string, TreeNode>): TreeItem[] => [...level.values()]
    .sort((a, b) => {
      const aFolder = a.children.size > 0;
      const bFolder = b.children.size > 0;
      if (aFolder !== bFolder) return aFolder ? -1 : 1;
      return a.name.localeCompare(b.name);
    })
    .map((node) => {
      if (node.children.size > 0) {
        const id = `dir:${node.path}`;
        folders.push(id);
        return { id, label: node.name, children: toItems(node.children) };
      }
      const id = `file:${node.path}`;
      files.push(id);
      if (node.change) byId.set(id, node.change);
      return { id, label: node.name };
    });
  return { items: toItems(root), folders, files, changes: byId };
}
