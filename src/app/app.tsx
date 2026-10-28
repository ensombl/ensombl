import { Bot, Code2, FolderOpen, Network, Plus } from "lucide-react"

import { Button } from "@/components/ui/button"
import { useAppInfo } from "@/features/app-info/queries/get-app-info"
import { Pane, PaneEmpty } from "@/shared/ui/pane"

import { AppShell } from "./layout/app-shell"
import { TitleBar } from "./layout/title-bar"

export function App() {
  const { data: info } = useAppInfo()

  return (
    <div className="flex h-full flex-col">
      <TitleBar version={info?.version} />
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
  )
}
