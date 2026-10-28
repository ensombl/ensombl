import { Bot, Plus } from "lucide-react"

import { Button } from "@/components/ui/button"
import { ArchitecturePane } from "@/features/architecture/ui/architecture-pane"
import { usePeek } from "@/features/peek/model/peek-store"
import { PeekPanel } from "@/features/peek/ui/peek-panel"
import { Pane, PaneEmpty } from "@/shared/ui/pane"

import { AppShell } from "../layout/app-shell"

/** Everything for one worktree tab: diagram, agents and peek. */
export function WorkspaceView() {
  const { target } = usePeek()

  return (
    <AppShell
      diagram={<ArchitecturePane />}
      peek={target && <PeekPanel target={target} />}
      agents={
        <Pane
          title="Agents"
          icon={<Bot />}
          actions={
            <Button variant="ghost" size="icon" className="size-7">
              <Plus />
            </Button>
          }
        >
          <PaneEmpty
            icon={<Bot />}
            title="No agent running"
            description="Start claude, codex or a custom CLI in this worktree. It uses your existing login."
          />
        </Pane>
      }
    />
  )
}
