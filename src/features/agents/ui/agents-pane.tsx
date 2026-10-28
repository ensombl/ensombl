import { useState } from "react"
import { Bot, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import type { TerminalInfo } from "@/shared/bindings/TerminalInfo"
import { Pane, PaneEmpty } from "@/shared/ui/pane"
import { useIsActiveWorkspace, useWorkspace } from "@/shared/workspace/workspace-context"

import { useModel } from "@/features/architecture/queries/model-queries"

import { useCloseTerminal, useStartTerminal } from "../commands/terminal-commands"
import { useAgents, useTerminals } from "../queries/agent-queries"
import { AgentIcon } from "./agent-icon"
import { Composer } from "./composer"
import { NewAgentMenu } from "./new-agent-menu"
import { TerminalView } from "./terminal-view"

/** Agent CLIs running in this worktree, one tab each. */
export function AgentsPane() {
  const worktree = useWorkspace()
  const workspaceActive = useIsActiveWorkspace()
  const { data: agents = [] } = useAgents()
  const { data: terminals = [] } = useTerminals(worktree.id)
  const start = useStartTerminal()
  const close = useCloseTerminal(worktree.id)
  const [selected, setSelected] = useState<string | null>(null)
  const { data: model = null } = useModel(worktree.id)

  const active = terminals.find((t) => t.id === selected) ?? terminals[terminals.length - 1]
  const kindOf = (t: TerminalInfo) => agents.find((a) => a.profile.id === t.profileId)?.profile.kind ?? "custom"

  const launch = (profileId: string) =>
    start.mutate({ worktreeId: worktree.id, profileId, cols: 100, rows: 30 }, { onSuccess: (info) => setSelected(info.id) })

  const closeTerminal = (id: string) => {
    close.mutate(id)
    if (id === active?.id) setSelected(null)
  }

  return (
    <Pane
      title="Agents"
      icon={<Bot />}
      actions={<NewAgentMenu agents={agents} onStart={launch} />}
    >
      {terminals.length === 0 ? (
        <PaneEmpty
          icon={<Bot />}
          title="No agent running"
          description="Agents run in this worktree with your existing CLI login. No API keys needed."
        >
          <div className="flex flex-wrap justify-center gap-2">
            {agents
              .filter((a) => a.available)
              .map(({ profile }) => (
                <Button key={profile.id} size="sm" variant="outline" onClick={() => launch(profile.id)} disabled={start.isPending}>
                  <AgentIcon kind={profile.kind} /> {profile.label}
                </Button>
              ))}
          </div>
        </PaneEmpty>
      ) : (
        <div className="flex h-full flex-col">
          <div role="tablist" className="flex h-9 shrink-0 items-center gap-1 overflow-x-auto border-b px-2">
            {terminals.map((t) => (
              <div
                key={t.id}
                role="tab"
                aria-selected={t.id === active?.id}
                onClick={() => setSelected(t.id)}
                className={cn(
                  "group flex h-7 shrink-0 cursor-default items-center gap-1.5 rounded-md pr-1 pl-2 text-xs",
                  t.id === active?.id ? "bg-accent text-accent-foreground" : "text-muted-foreground hover:bg-accent/50",
                )}
              >
                <AgentIcon kind={kindOf(t)} className="size-3.5" />
                <span className={cn(!t.running && "line-through opacity-60")}>{t.title}</span>
                <button
                  type="button"
                  aria-label={`Close ${t.title}`}
                  onClick={(e) => {
                    e.stopPropagation()
                    closeTerminal(t.id)
                  }}
                  className="flex size-4 items-center justify-center rounded-sm opacity-0 group-hover:opacity-100 hover:bg-muted-foreground/20"
                >
                  <X className="size-3" />
                </button>
              </div>
            ))}
          </div>
          <div className="relative min-h-0 flex-1">
            {terminals.map((t) => (
              <TerminalView
                key={t.id}
                terminalId={t.id}
                visible={workspaceActive && t.id === active?.id}
                onRestart={() => {
                  closeTerminal(t.id)
                  launch(t.profileId)
                }}
                onClose={() => closeTerminal(t.id)}
              />
            ))}
          </div>
          <Composer model={model} target={active} />
        </div>
      )}
    </Pane>
  )
}
