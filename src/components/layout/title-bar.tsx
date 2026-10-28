import { Boxes } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Separator } from "@/components/ui/separator"

export function TitleBar({ version }: { version?: string }) {
  return (
    <header className="flex h-11 shrink-0 items-center gap-3 border-b bg-sidebar px-4">
      <div className="flex items-center gap-2">
        <div className="flex size-6 items-center justify-center rounded-md bg-primary text-primary-foreground">
          <Boxes className="size-3.5" />
        </div>
        <span className="text-sm font-semibold">CRM</span>
      </div>
      <Separator orientation="vertical" className="h-4!" />
      <span className="text-sm text-muted-foreground">Clanker Resource Management</span>
      {version && (
        <Badge variant="outline" className="ml-auto font-mono">
          v{version}
        </Badge>
      )}
    </header>
  )
}
