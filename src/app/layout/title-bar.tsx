import type { ReactNode } from "react"

import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"

// macOS draws the traffic lights over this bar (titleBarStyle: Overlay), so leave room for them.
const isMac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac")

export function TitleBar({ version, children }: { version?: string; children?: ReactNode }) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        "flex h-9 shrink-0 items-center gap-2 border-b bg-sidebar pr-2 select-none",
        isMac ? "pl-[76px]" : "pl-2",
      )}
    >
      {children}
      {version && (
        <Badge variant="outline" className="ml-auto h-5 shrink-0 px-1.5 font-mono text-[10px]">
          v{version}
        </Badge>
      )}
    </header>
  )
}
