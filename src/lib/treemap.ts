export type Rect = { x: number; y: number; w: number; h: number };

/** Worst aspect ratio of a row of areas laid along a side of length `side`. */
function worst(row: number[], side: number): number {
  const sum = row.reduce((a, b) => a + b, 0);
  const max = Math.max(...row);
  const min = Math.min(...row);
  return Math.max((side * side * max) / (sum * sum), (sum * sum) / (side * side * min));
}

/** Squarified treemap (Bruls et al.). `values` must be sorted largest first and > 0. */
export function squarify(values: number[], rect: Rect): Rect[] {
  const total = values.reduce((a, b) => a + b, 0);
  if (!total || rect.w <= 0 || rect.h <= 0) return [];
  const scale = (rect.w * rect.h) / total;
  const areas = values.map((v) => v * scale);
  let { x, y, w, h } = rect;
  const out: Rect[] = [];
  let i = 0;
  while (i < areas.length) {
    const side = Math.min(w, h);
    const row = [areas[i]];
    let j = i + 1;
    while (j < areas.length && worst([...row, areas[j]], side) <= worst(row, side)) row.push(areas[j++]);
    const sum = row.reduce((a, b) => a + b, 0);
    if (w >= h) {
      const cw = sum / h;
      let cy = y;
      for (const a of row) {
        out.push({ x, y: cy, w: cw, h: a / cw });
        cy += a / cw;
      }
      x += cw;
      w -= cw;
    } else {
      const rh = sum / w;
      let cx = x;
      for (const a of row) {
        out.push({ x: cx, y, w: a / rh, h: rh });
        cx += a / rh;
      }
      y += rh;
      h -= rh;
    }
    i = j;
  }
  return out;
}
