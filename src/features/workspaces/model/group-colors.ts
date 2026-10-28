import type { GroupColor } from "@/shared/bindings/GroupColor"

/** Tailwind classes per tab-group color: chip fill, and the accent line under grouped tabs. */
export const groupColors: Record<GroupColor, { chip: string; line: string; dot: string }> = {
  blue: { chip: "bg-blue-500/20 text-blue-200 hover:bg-blue-500/30", line: "bg-blue-500", dot: "bg-blue-500" },
  green: { chip: "bg-emerald-500/20 text-emerald-200 hover:bg-emerald-500/30", line: "bg-emerald-500", dot: "bg-emerald-500" },
  purple: { chip: "bg-violet-500/20 text-violet-200 hover:bg-violet-500/30", line: "bg-violet-500", dot: "bg-violet-500" },
  orange: { chip: "bg-orange-500/20 text-orange-200 hover:bg-orange-500/30", line: "bg-orange-500", dot: "bg-orange-500" },
  pink: { chip: "bg-pink-500/20 text-pink-200 hover:bg-pink-500/30", line: "bg-pink-500", dot: "bg-pink-500" },
  cyan: { chip: "bg-cyan-500/20 text-cyan-200 hover:bg-cyan-500/30", line: "bg-cyan-500", dot: "bg-cyan-500" },
  yellow: { chip: "bg-yellow-500/20 text-yellow-200 hover:bg-yellow-500/30", line: "bg-yellow-500", dot: "bg-yellow-500" },
  red: { chip: "bg-red-500/20 text-red-200 hover:bg-red-500/30", line: "bg-red-500", dot: "bg-red-500" },
  grey: { chip: "bg-zinc-500/20 text-zinc-200 hover:bg-zinc-500/30", line: "bg-zinc-500", dot: "bg-zinc-500" },
}
