import type { ReactNode } from "react"
import { Plus } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import type { AgentAvailability } from "@/shared/bindings/AgentAvailability"

import { AgentIcon } from "./agent-icon"

interface NewAgentMenuProps {
  agents: AgentAvailability[]
  onStart: (profileId: string) => void
  trigger?: ReactNode
}

export function NewAgentMenu({ agents, onStart, trigger }: NewAgentMenuProps) {
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
        {agents.map(({ profile, available, path }) => (
          <DropdownMenuItem
            key={profile.id}
            disabled={!available}
            onSelect={() => onStart(profile.id)}
            title={path ?? `${profile.command} not found on your PATH`}
          >
            <AgentIcon kind={profile.kind} />
            {profile.label}
            {!available && <span className="ml-auto text-xs text-muted-foreground">not installed</span>}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
