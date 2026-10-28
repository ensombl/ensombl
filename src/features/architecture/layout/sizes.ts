import type { Member } from "@/shared/bindings/Member"

import type { ViewNode } from "../model/view"

/** Node dimensions shared by the layout engine and the node components. */
export const SIZES = {
  headerHeight: 44,
  rowHeight: 22,
  sectionGap: 9,
  charWidth: 6.7,
  symbolMinWidth: 170,
  symbolMaxWidth: 560,
  packageCollapsed: { width: 240, height: 76 },
  folderCollapsed: { width: 210, height: 58 },
  groupPadding: { top: 46, side: 20, bottom: 20 },
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
  const { symbol, open } = node
  const titleChars = symbol.name.length + (symbol.typeParams?.length ?? 0)
  let width = Math.max(SIZES.symbolMinWidth, titleChars * 8 + 84)
  let height = SIZES.headerHeight
  if (open && symbol.members.length > 0) {
    const { fields, methods } = splitMembers(symbol.members)
    const longest = Math.max(...symbol.members.map((m) => m.signature.length + 4))
    width = Math.max(width, longest * SIZES.charWidth + 40)
    height += (fields.length + methods.length) * SIZES.rowHeight + (fields.length && methods.length ? 2 : 1) * SIZES.sectionGap
  }
  return { width: Math.min(Math.ceil(width), SIZES.symbolMaxWidth), height }
}
