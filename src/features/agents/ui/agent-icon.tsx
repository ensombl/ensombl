import { Bot, Sparkles, SquareTerminal, Terminal } from "lucide-react"

import type { AgentKind } from "@/shared/bindings/AgentKind"

export function AgentIcon({ kind, className }: { kind: AgentKind; className?: string }) {
  const Icon = { claude: Sparkles, codex: Bot, shell: SquareTerminal, custom: Terminal }[kind]
  return <Icon className={className} />
}
