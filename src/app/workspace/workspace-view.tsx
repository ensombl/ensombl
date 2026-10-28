import { AgentsPane } from "@/features/agents/ui/agents-pane"
import { ArchitecturePane } from "@/features/architecture/ui/architecture-pane"
import { usePeek } from "@/features/peek/model/peek-store"
import { PeekPanel } from "@/features/peek/ui/peek-panel"

import { AppShell } from "../layout/app-shell"

/** Everything for one worktree tab: diagram, agents and peek. */
export function WorkspaceView() {
  const { target } = usePeek()
  return <AppShell diagram={<ArchitecturePane />} peek={target && <PeekPanel target={target} />} agents={<AgentsPane />} />
}
