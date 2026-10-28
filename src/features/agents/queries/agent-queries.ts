import { useQuery } from "@tanstack/react-query"

import type { AgentAvailability } from "@/shared/bindings/AgentAvailability"
import type { TerminalInfo } from "@/shared/bindings/TerminalInfo"
import { query } from "@/shared/ipc/client"

export const agentKeys = {
  all: ["agents"] as const,
  available: () => [...agentKeys.all, "available"] as const,
  terminals: (worktreeId: string) => [...agentKeys.all, "terminals", worktreeId] as const,
}

export function useAgents() {
  return useQuery({
    queryKey: agentKeys.available(),
    queryFn: () => query<AgentAvailability[]>("list_agents"),
    staleTime: 60_000,
  })
}

export function useTerminals(worktreeId: string) {
  return useQuery({
    queryKey: agentKeys.terminals(worktreeId),
    queryFn: () => query<TerminalInfo[]>("list_terminals", { input: { worktreeId } }),
  })
}

