import { useEffect, useState } from "react"
import { Bot, Code2, FolderOpen, Network, Plus } from "lucide-react"

import { AppShell } from "@/components/layout/app-shell"
import { Pane, PaneEmpty } from "@/components/layout/pane"
import { TitleBar } from "@/components/layout/title-bar"
import { Button } from "@/components/ui/button"
import { TooltipProvider } from "@/components/ui/tooltip"
import { ipc } from "@/ipc"

export default function App() {
  const [version, setVersion] = useState<string>()

  useEffect(() => {
    ipc
      .appInfo()
      .then((info) => setVersion(info.version))
      .catch(() => setVersion(undefined))
  }, [])

  return (
    <TooltipProvider>
      <div className="flex h-full flex-col">
        <TitleBar version={version} />
        <AppShell
          diagram={
            <Pane title="Architecture" icon={<Network />}>
              <PaneEmpty
                icon={<Network />}
                title="No repository open"
                description="Open a TypeScript or JavaScript repository to see its whole architecture as UML."
              >
                <Button size="sm">
                  <FolderOpen /> Open repository
                </Button>
              </PaneEmpty>
            </Pane>
          }
          peek={
            <Pane title="Peek" icon={<Code2 />}>
              <PaneEmpty
                icon={<Code2 />}
                title="Nothing to peek"
                description="Click a member on the diagram to see its code."
              />
            </Pane>
          }
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
                description="Start claude, codex or a custom CLI. It uses your existing login."
              />
            </Pane>
          }
        />
      </div>
    </TooltipProvider>
  )
}
