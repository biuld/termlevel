/** Pure workspace geometry. It never owns a terminal process or view. */
export type Direction = "left" | "right" | "up" | "down";
export type DropPosition = Direction | "center";
export type Pane = { kind: "pane"; id: string; sessionId: string };
export type Split = { kind: "split"; id: string; axis: "row" | "column"; ratio: number; first: LayoutNode; second: LayoutNode };
export type LayoutNode = Pane | Split;
export type Rect = { x: number; y: number; width: number; height: number };
export type Size = { width: number; height: number };
export type TilingState = { root: LayoutNode | null; focusedPaneId: string | null; zoomed: boolean };
export type Divider = { id: string; axis: "row" | "column"; rect: Rect; parent: Rect; minRatio: number; maxRatio: number };

export function panes(root: LayoutNode | null): Pane[] {
  return !root ? [] : root.kind === "pane" ? [root] : [...panes(root.first), ...panes(root.second)];
}
export function updateNode(root: LayoutNode, id: string, fn: (node: LayoutNode) => LayoutNode): LayoutNode {
  if (root.id === id) return fn(root);
  return root.kind === "pane" ? root : { ...root, first: updateNode(root.first, id, fn), second: updateNode(root.second, id, fn) };
}
export function removePane(root: LayoutNode | null, id: string): LayoutNode | null {
  if (!root || root.id === id) return null;
  if (root.kind === "pane") return root;
  const first = removePane(root.first, id), second = removePane(root.second, id);
  return !first ? second : !second ? first : { ...root, first, second };
}
export function siblings(root: LayoutNode | null, id: string): Pane[] {
  if (!root || root.kind === "pane") return [];
  if (root.first.id === id) return panes(root.second);
  if (root.second.id === id) return panes(root.first);
  return [...siblings(root.first, id), ...siblings(root.second, id)];
}
export function insertPane(root: LayoutNode, targetId: string, pane: Pane, direction: Direction, splitId: string): LayoutNode {
  return updateNode(root, targetId, target => {
    const before = direction === "left" || direction === "up";
    return { kind: "split", id: splitId, axis: direction === "left" || direction === "right" ? "row" : "column", ratio: 0.5,
      first: before ? pane : target, second: before ? target : pane };
  });
}
export function minimum(root: LayoutNode | null, cell: Size, gap: number): Size {
  if (!root) return { width: 0, height: 0 };
  if (root.kind === "pane") return cell;
  const a = minimum(root.first, cell, gap), b = minimum(root.second, cell, gap);
  return root.axis === "row" ? { width: a.width + b.width + gap, height: Math.max(a.height, b.height) }
    : { width: Math.max(a.width, b.width), height: a.height + b.height + gap };
}
export function geometry(root: LayoutNode | null, rect: Rect, cell: Size, gap: number) {
  const frames = new Map<string, Rect>(), dividers: Divider[] = [];
  function walk(node: LayoutNode, box: Rect) {
    if (node.kind === "pane") { frames.set(node.id, box); return; }
    const row = node.axis === "row", extent = Math.max(0, (row ? box.width : box.height) - gap);
    const a = minimum(node.first, cell, gap), b = minimum(node.second, cell, gap);
    const minRatio = extent > 0 ? (row ? a.width : a.height) / extent : 0.5;
    const maxRatio = extent > 0 ? 1 - (row ? b.width : b.height) / extent : 0.5;
    const ratio = minRatio <= maxRatio ? Math.max(minRatio, Math.min(maxRatio, node.ratio)) : node.ratio;
    const first = Math.round(extent * ratio), second = Math.max(0, extent - first);
    const divider = row ? { x: box.x + first, y: box.y, width: gap, height: box.height }
      : { x: box.x, y: box.y + first, width: box.width, height: gap };
    dividers.push({ id: node.id, axis: node.axis, rect: divider, parent: box, minRatio, maxRatio });
    walk(node.first, row ? { ...box, width: first } : { ...box, height: first });
    walk(node.second, row ? { ...box, x: box.x + first + gap, width: second } : { ...box, y: box.y + first + gap, height: second });
  }
  if (root) walk(root, rect);
  return { frames, dividers };
}
export function neighbor(id: string, direction: Direction, frames: Map<string, Rect>): string | null {
  const from = frames.get(id);
  if (!from) return null;
  const row = direction === "left" || direction === "right", positive = direction === "right" || direction === "down";
  const center = (r: Rect) => row ? r.x + r.width / 2 : r.y + r.height / 2;
  const perpendicular = (r: Rect) => row ? r.y + r.height / 2 : r.x + r.width / 2;
  const candidates = [...frames].filter(([other, r]) => other !== id && (positive ? center(r) > center(from) : center(r) < center(from)));
  const score = (r: Rect) => {
    const overlap = row ? Math.min(from.y + from.height, r.y + r.height) - Math.max(from.y, r.y)
      : Math.min(from.x + from.width, r.x + r.width) - Math.max(from.x, r.x);
    const gap = row ? (positive ? r.x - from.x - from.width : from.x - r.x - r.width)
      : (positive ? r.y - from.y - from.height : from.y - r.y - r.height);
    return [overlap > 0 ? 0 : 1, Math.max(0, gap), Math.abs(perpendicular(r) - perpendicular(from))];
  };
  candidates.sort(([ai, a], [bi, b]) => { const x = score(a), y = score(b); return x[0] - y[0] || x[1] - y[1] || x[2] - y[2] || ai.localeCompare(bi); });
  return candidates[0]?.[0] ?? null;
}
export function equalize(root: LayoutNode): LayoutNode {
  return root.kind === "pane" ? root : { ...root, ratio: 0.5, first: equalize(root.first), second: equalize(root.second) };
}
export function dropPosition(rect: Rect, x: number, y: number): DropPosition {
  const dx = (x - rect.x) / rect.width, dy = (y - rect.y) / rect.height;
  const edges: [Direction, number][] = [["left", dx], ["right", 1 - dx], ["up", dy], ["down", 1 - dy]];
  edges.sort((a, b) => a[1] - b[1]);
  return edges[0][1] < 0.24 ? edges[0][0] : "center";
}
