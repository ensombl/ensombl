import { Position, type NodeHandle } from "@xyflow/react"

/*
 * Handles declared up front. React Flow drops a node's measured handles
 * whenever it receives a new node object without `measured`, and edges
 * to that node vanish until something happens to re-measure it. Declaring
 * the handles on every node makes edges independent of DOM measuring.
 */

/** Target on the left, source on the right (left-to-right maps). */
export const sideHandles = (width: number, height: number): NodeHandle[] => [
  { type: "target", position: Position.Left, x: 0, y: height / 2, width: 1, height: 1 },
  { type: "source", position: Position.Right, x: width - 1, y: height / 2, width: 1, height: 1 },
]

/** Source on top, target at the bottom (dependencies point up). */
export const verticalHandles = (width: number, height: number): NodeHandle[] => [
  { type: "source", position: Position.Top, x: width / 2, y: 0, width: 1, height: 1 },
  { type: "target", position: Position.Bottom, x: width / 2, y: height - 1, width: 1, height: 1 },
]
