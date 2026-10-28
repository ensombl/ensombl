import type { Point } from "./elk-graph"

/**
 * Polyline through route points with rounded corners. Also returns where a
 * label fits best: the middle of the longest segment (away from bends, nodes
 * and arrowheads) and that segment's length.
 */
export function roundedPath(points: Point[], radius = 6): [string, number, number, number] {
  let d = `M ${points[0].x} ${points[0].y}`
  for (let i = 1; i < points.length - 1; i++) {
    const [prev, cur, next] = [points[i - 1], points[i], points[i + 1]]
    const r = Math.min(radius, Math.hypot(cur.x - prev.x, cur.y - prev.y) / 2, Math.hypot(next.x - cur.x, next.y - cur.y) / 2)
    const inX = cur.x - Math.sign(cur.x - prev.x) * r
    const inY = cur.y - Math.sign(cur.y - prev.y) * r
    const outX = cur.x + Math.sign(next.x - cur.x) * r
    const outY = cur.y + Math.sign(next.y - cur.y) * r
    d += ` L ${inX} ${inY} Q ${cur.x} ${cur.y} ${outX} ${outY}`
  }
  const last = points[points.length - 1]
  d += ` L ${last.x} ${last.y}`
  let best = 0
  let length = -1
  for (let i = 0; i < points.length - 1; i++) {
    const len = Math.hypot(points[i + 1].x - points[i].x, points[i + 1].y - points[i].y)
    if (len > length) {
      length = len
      best = i
    }
  }
  const [a, b] = [points[best], points[best + 1]]
  return [d, (a.x + b.x) / 2, (a.y + b.y) / 2, length]
}
