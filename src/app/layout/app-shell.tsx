import type { ReactNode } from "react"

import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable"

interface AppShellProps {
  diagram: ReactNode
  agents: ReactNode
  /** Rendered as a bottom drawer only while something is being peeked. */
  peek?: ReactNode
}

/** The diagram is always on screen, with agents on the right and peek sliding up on demand. */
export function AppShell({ diagram, agents, peek }: AppShellProps) {
  return (
    <ResizablePanelGroup orientation="horizontal" className="min-h-0 flex-1">
      <ResizablePanel defaultSize="68" minSize="30">
        <ResizablePanelGroup orientation="vertical">
          <ResizablePanel id="diagram" minSize="20">
            {diagram}
          </ResizablePanel>
          {peek && (
            <>
              <ResizableHandle withHandle />
              <ResizablePanel
                id="peek"
                defaultSize="35"
                minSize="15"
                className="animate-in slide-in-from-bottom-4 fade-in-0 duration-200"
              >
                {peek}
              </ResizablePanel>
            </>
          )}
        </ResizablePanelGroup>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize="32" minSize="20">
        {agents}
      </ResizablePanel>
    </ResizablePanelGroup>
  )
}
