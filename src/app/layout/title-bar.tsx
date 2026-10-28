import { Boxes } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"

// macOS draws the traffic lights over this bar (titleBarStyle: Overlay), so leave room for them.
const isMac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac")

export function TitleBar({ version }: { version?: string }) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        "flex h-[52px] shrink-0 items-center gap-2 border-b bg-sidebar pr-4 select-none",
        isMac ? "pl-[100px]" : "pl-4",
      )}
    >
      <div data-tauri-drag-region className="pointer-events-none flex items-center gap-2">
        <div className="flex size-6 items-center justify-center rounded-md bg-primary text-primary-foreground">
          <Boxes className="size-3.5" />
        </div>
        <span className="text-sm font-semibold" title="Clanker Resource Management">CRM</span>
      </div>
      {version && (
        <Badge variant="outline" className="ml-auto font-mono">
          v{version}
        </Badge>
      )}
    </header>
  )
}
