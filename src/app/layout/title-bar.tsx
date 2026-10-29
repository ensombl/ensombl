import type { ReactNode } from "react"

import { VersionMenu } from "@/features/updates/ui/version-menu"
import { cn } from "@/lib/utils"

// macOS draws the traffic lights over this bar (titleBarStyle: Overlay), so leave room for them.
const isMac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac")

export function TitleBar({ version, children }: { version?: string; children?: ReactNode }) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        "flex h-10 shrink-0 items-center gap-2 border-b bg-sidebar pr-2 select-none",
        isMac ? "pl-[86px]" : "pl-2",
      )}
    >
      {children}
      {version && <VersionMenu version={version} />}
    </header>
  )
}
