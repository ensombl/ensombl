import { Toaster } from "@/components/ui/sonner"
import { useAppInfo } from "@/features/app-info/queries/get-app-info"
import { useSession } from "@/features/workspaces/queries/workspace-queries"
import { TabStrip } from "@/features/workspaces/ui/tab-strip"
import { cn } from "@/lib/utils"
import { WorkspaceProvider } from "@/shared/workspace/workspace-context"

import { TitleBar } from "./layout/title-bar"
import { Welcome } from "./workspace/welcome"
import { WorkspaceView } from "./workspace/workspace-view"

export function App() {
  const { data: info } = useAppInfo()
  const { data: session } = useSession()
  const tabs = session?.groups.flatMap((g) => g.tabs) ?? []

  return (
    <div className="flex h-full flex-col">
      <TitleBar version={info?.version}>{session && <TabStrip session={session} />}</TitleBar>
      <main className="relative min-h-0 flex-1">
        {session && tabs.length === 0 && <Welcome />}
        {/* Every open tab stays mounted so its terminals and diagram survive tab switches. */}
        {tabs.map(({ worktree }) => (
          <div
            key={worktree.id}
            className={cn("absolute inset-0 flex", worktree.id !== session?.active && "invisible")}
          >
            <WorkspaceProvider worktree={worktree} active={worktree.id === session?.active}>
              <WorkspaceView />
            </WorkspaceProvider>
          </div>
        ))}
      </main>
      <Toaster position="bottom-right" />
    </div>
  )
}
