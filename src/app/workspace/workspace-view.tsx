import { Bot, Network, Plus } from "lucide-react"

import { Button } from "@/components/ui/button"
import { usePeek } from "@/features/peek/model/peek-store"
import { PeekPanel } from "@/features/peek/ui/peek-panel"
import { Pane, PaneEmpty } from "@/shared/ui/pane"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { AppShell } from "../layout/app-shell"

/** Everything for one worktree tab: diagram, agents and peek. */
export function WorkspaceView() {
  const worktree = useWorkspace()
  const { target } = usePeek()

  return (
    <AppShell
      diagram={
        <Pane title="Architecture" icon={<Network />}>
          <PaneEmpty
            icon={<Network />}
            title="Indexing comes next"
            description={`The UML for ${worktree.path} will render here.`}
          />
        </Pane>
      }
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
