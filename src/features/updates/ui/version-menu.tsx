import { useEffect, useRef } from "react"
import { ArrowDownToLine, Loader2, RefreshCw } from "lucide-react"
import { toast } from "sonner"

import { Badge } from "@/components/ui/badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { cn } from "@/lib/utils"

import { useAvailableUpdate, useInstallUpdate } from "../queries/updates"

/** The running version; offers a newer release when one is out. */
export function VersionMenu({ version }: { version: string }) {
  const available = useAvailableUpdate()
  const install = useInstallUpdate()
  const update = available.data ?? null
  const announced = useRef<string | null>(null)

  const restart = () => {
    if (!update) return
    const id = toast.loading(`Downloading Ensombl ${update.version}…`)
    install.mutate(update, {
      onError: (e) => toast.error(`Couldn't update: ${String(e)}`, { id }),
    })
  }

  useEffect(() => {
    if (!update || announced.current === update.version) return
    announced.current = update.version
    toast(`Ensombl ${update.version} is available`, {
      description: "It installs in the background and Ensombl restarts. Running agents stop.",
      duration: Infinity,
      action: { label: "Restart to update", onClick: restart },
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [update])

  const checkNow = async () => {
    const { data, error } = await available.refetch()
    if (error) toast.error(`Couldn't check for updates: ${String(error)}`)
    else if (!data) toast.success(`You're on the latest version (v${version})`)
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button type="button" className="ml-auto shrink-0 rounded-md outline-none focus-visible:ring-2 focus-visible:ring-ring">
          <Badge
            variant="outline"
            className={cn("h-5 cursor-default px-1.5 font-mono text-[10px]", update && "border-primary/60 bg-primary/10 text-primary")}
          >
            {update ? <ArrowDownToLine className="size-3" /> : null}v{version}
          </Badge>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-60">
        <DropdownMenuLabel className="font-normal">
          Ensombl <span className="font-mono text-xs text-muted-foreground">v{version}</span>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {update && (
          <DropdownMenuItem onSelect={restart} disabled={install.isPending}>
            {install.isPending ? <Loader2 className="animate-spin" /> : <ArrowDownToLine />}
            Restart to update to v{update.version}
          </DropdownMenuItem>
        )}
        <DropdownMenuItem onSelect={checkNow} disabled={available.isFetching}>
          {available.isFetching ? <Loader2 className="animate-spin" /> : <RefreshCw />}
          Check for updates
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
