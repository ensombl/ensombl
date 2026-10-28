import { OctagonX } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"

import { useUpdateSettings } from "@/features/peek/commands/peek-commands"
import { useSettings } from "@/features/peek/queries/peek-queries"

import { useTaskCommands } from "../commands/task-commands"
import { useUsage } from "../queries/task-queries"

const LIMITS = [1, 2, 3, 4, 6, 8]

const dollars = (n: number) => `$${n < 10 ? n.toFixed(2) : n.toFixed(0)}`

/** Spend so far, how many agents may run at once, and a stop switch. */
export function RunControls({ projectId, running }: { projectId: string; running: number }) {
  const { data: usage } = useUsage(projectId)
  const { data: settings } = useSettings()
  const updateSettings = useUpdateSettings()
  const { stopAll } = useTaskCommands()
  return (
    <div className="flex items-center gap-2 text-xs text-muted-foreground">
      {usage && (
        <span title={`${usage.runsToday} runs today, ${usage.runs} in all, ${dollars(usage.totalUsd)} in all`}>
          {dollars(usage.todayUsd)} today
        </span>
      )}
      {settings && (
        <Select
          value={String(settings.taskConcurrency)}
          onValueChange={(v) => updateSettings.mutate({ ...settings, taskConcurrency: Number(v) })}
        >
          <SelectTrigger size="sm" className="h-7 w-auto gap-1 text-xs" aria-label="Agents at once">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {LIMITS.map((n) => (
              <SelectItem key={n} value={String(n)} className="text-xs">
                {n} at once
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      )}
      {running > 0 && (
        <Button size="sm" variant="ghost" className="h-7 text-xs text-red-300" onClick={() => stopAll.mutate({ projectId })}>
          <OctagonX /> Stop all
        </Button>
      )}
    </div>
  )
}
