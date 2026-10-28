import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import type { PermissionMode } from "@/shared/bindings/PermissionMode"

const MODES: { value: PermissionMode; label: string }[] = [
  { value: "default", label: "Ask before edits" },
  { value: "acceptEdits", label: "Accept edits" },
  { value: "plan", label: "Plan only" },
  { value: "bypassPermissions", label: "Bypass permissions" },
]

const MODELS = ["opus", "sonnet", "haiku"] as const

/** Maps a full model id (`claude-opus-…`) to the alias the picker shows. */
const modelAlias = (model: string | null) => MODELS.find((m) => model?.includes(m)) ?? ""

interface ChatToolbarProps {
  mode: PermissionMode
  model: string | null
  costUsd: number | null
  disabled: boolean
  onMode: (mode: PermissionMode) => void
  onModel: (model: string) => void
}

export function ChatToolbar({ mode, model, costUsd, disabled, onMode, onModel }: ChatToolbarProps) {
  const alias = modelAlias(model)
  return (
    <div className="flex h-10 shrink-0 items-center gap-2 border-b px-2">
      <Select value={mode} onValueChange={(v) => onMode(v as PermissionMode)} disabled={disabled}>
        <SelectTrigger size="sm" className="h-7 text-xs" aria-label="Permission mode">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {MODES.map((m) => (
            <SelectItem key={m.value} value={m.value} className="text-xs">
              {m.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Select value={alias} onValueChange={onModel} disabled={disabled}>
        <SelectTrigger size="sm" className="h-7 text-xs capitalize" aria-label="Model">
          <SelectValue placeholder={model ?? "Model"} />
        </SelectTrigger>
        <SelectContent>
          {MODELS.map((m) => (
            <SelectItem key={m} value={m} className="text-xs capitalize">
              {m}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <div className="ml-auto font-mono text-[11px] text-muted-foreground">
        {costUsd !== null && <span title="Session cost">${costUsd.toFixed(2)}</span>}
      </div>
    </div>
  )
}
