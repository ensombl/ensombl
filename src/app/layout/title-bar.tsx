import type { ReactNode } from "react"
import { Boxes } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"

// macOS draws the traffic lights over this bar (titleBarStyle: Overlay), so leave room for them.
const isMac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac")

export function TitleBar({ version, children }: { version?: string; children?: ReactNode }) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        "flex h-[52px] shrink-0 items-center gap-3 border-b bg-sidebar pr-3 select-none",
        isMac ? "pl-[92px]" : "pl-3",
      )}
    >
      <Tooltip>
        <TooltipTrigger asChild>
          <div className="flex size-6 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <Boxes className="size-3.5" />
          </div>
        </TooltipTrigger>
        <TooltipContent>CRM: Clanker Resource Management</TooltipContent>
      </Tooltip>
      {children}
      {version && (
        <Badge variant="outline" className="ml-auto shrink-0 font-mono">
          v{version}
        </Badge>
      )}
    </header>
  )
}
