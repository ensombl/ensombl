import type { ReactNode } from "react"

import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable"

interface AppShellProps {
  diagram: ReactNode
  agents: ReactNode
  peek: ReactNode
}

/** The diagram is always on screen, with agents on the right and the peek drawer below. */
export function AppShell({ diagram, agents, peek }: AppShellProps) {
  return (
    <ResizablePanelGroup orientation="horizontal" className="min-h-0 flex-1">
      <ResizablePanel defaultSize="68" minSize="30">
        <ResizablePanelGroup orientation="vertical">
          <ResizablePanel defaultSize="72" minSize="20">
            {diagram}
          </ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel defaultSize="28" minSize="0" collapsible>
            {peek}
          </ResizablePanel>
        </ResizablePanelGroup>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize="32" minSize="20">
        {agents}
      </ResizablePanel>
    </ResizablePanelGroup>
  )
}
