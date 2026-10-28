import type { Member } from "@/shared/bindings/Member"

import type { ViewNode } from "../model/view"

/** Node dimensions shared by the layout engine and the node components. */
export const SIZES = {
  symbolHeight: 40,
  symbolMinWidth: 150,
  symbolMaxWidth: 280,
  packageCollapsed: { width: 230, height: 64 },
  folderCollapsed: { width: 200, height: 52 },
  groupPadding: { top: 44, side: 18, bottom: 18 },
}

export function splitMembers(members: Member[]) {
  const fields = members.filter((m) => m.kind === "property" || m.kind === "enumMember")
  const methods = members.filter((m) => m.kind !== "property" && m.kind !== "enumMember")
  return { fields, methods }
}

export function nodeSize(node: ViewNode): { width: number; height: number } {
  if (node.type === "group") {
    return node.level === "package" ? SIZES.packageCollapsed : SIZES.folderCollapsed
  }
  // Compact by design: members live in the inspector, not on the map.
  const chars = node.symbol.name.length + Math.min(node.symbol.typeParams?.length ?? 0, 12)
  const width = Math.min(Math.max(SIZES.symbolMinWidth, chars * 7.4 + 76), SIZES.symbolMaxWidth)
  return { width: Math.ceil(width), height: SIZES.symbolHeight }
}
