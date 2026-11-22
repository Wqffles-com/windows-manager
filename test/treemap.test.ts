// Run with: pnpm test
import { squarify } from "../src/lib/treemap.ts";

const values = [6, 6, 4, 3, 2, 2, 1];
const rects = squarify(values, { x: 0, y: 0, w: 6, h: 4 });
const eps = 1e-9;
const check = (ok: boolean, msg: string) => {
  if (!ok) throw new Error(msg);
};

check(rects.length === values.length, "one rect per value");
rects.forEach((r, i) => {
  check(Math.abs(r.w * r.h - values[i]) < eps, `area of ${i} matches its value`);
  check(r.x >= -eps && r.y >= -eps && r.x + r.w <= 6 + eps && r.y + r.h <= 4 + eps, `${i} inside bounds`);
  rects.slice(i + 1).forEach((o, j) => {
    const overlap = Math.min(r.x + r.w, o.x + o.w) - Math.max(r.x, o.x) > eps && Math.min(r.y + r.h, o.y + o.h) - Math.max(r.y, o.y) > eps;
    check(!overlap, `${i} and ${i + j + 1} do not overlap`);
  });
});
check(squarify([], { x: 0, y: 0, w: 10, h: 10 }).length === 0, "empty input");
console.log("treemap ok");
