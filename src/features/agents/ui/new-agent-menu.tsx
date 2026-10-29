import type { ReactNode } from "react"
import { Crown, Plus } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import type { AgentAvailability } from "@/shared/bindings/AgentAvailability"

import { AgentIcon } from "./agent-icon"

interface NewAgentMenuProps {
  agents: AgentAvailability[]
  onStart: (profileId: string, lead: boolean) => void
  trigger?: ReactNode
}

function AgentItem({ agent, onSelect }: { agent: AgentAvailability; onSelect: () => void }) {
  const { profile, available, path } = agent
  return (
    <DropdownMenuItem disabled={!available} onSelect={onSelect} title={path ?? `${profile.command} not found on your PATH`}>
      <AgentIcon kind={profile.kind} />
      {profile.label}
      {!available && <span className="ml-auto text-xs text-muted-foreground">not installed</span>}
    </DropdownMenuItem>
  )
}

export function NewAgentMenu({ agents, onStart, trigger }: NewAgentMenuProps) {
  const leaders = agents.filter((a) => a.profile.leadArgs)
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        {trigger ?? (
          <Button variant="ghost" size="icon" className="size-7" aria-label="New agent">
            <Plus />
          </Button>
        )}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-60">
        <DropdownMenuLabel>Start in this worktree</DropdownMenuLabel>
        {agents.map((agent) => (
          <AgentItem key={agent.profile.id} agent={agent} onSelect={() => onStart(agent.profile.id, false)} />
        ))}
        {leaders.length > 0 && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuSub>
              <DropdownMenuSubTrigger title="An agent that splits your request into tasks, runs them and merges the results">
                <Crown />
                Start a lead
              </DropdownMenuSubTrigger>
              <DropdownMenuSubContent className="w-52">
                <DropdownMenuLabel className="text-xs font-normal text-muted-foreground">Who leads?</DropdownMenuLabel>
                {leaders.map((agent) => (
                  <AgentItem key={agent.profile.id} agent={agent} onSelect={() => onStart(agent.profile.id, true)} />
                ))}
              </DropdownMenuSubContent>
            </DropdownMenuSub>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
