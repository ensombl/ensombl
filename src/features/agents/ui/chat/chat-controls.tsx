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

// Quiet triggers that sit inside the composer.
const TRIGGER = "h-7 gap-1 border-0 bg-transparent px-2 text-xs text-muted-foreground shadow-none hover:text-foreground dark:bg-transparent"

interface ChatControlsProps {
  mode: PermissionMode
  model: string | null
  disabled: boolean
  onMode: (mode: PermissionMode) => void
  onModel: (model: string) => void
}

/** Permission mode and model pickers for the composer's bottom row. */
export function ChatControls({ mode, model, disabled, onMode, onModel }: ChatControlsProps) {
  return (
    <>
      <Select value={mode} onValueChange={(v) => onMode(v as PermissionMode)} disabled={disabled}>
        <SelectTrigger size="sm" className={TRIGGER} aria-label="Permission mode">
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
      <Select value={modelAlias(model)} onValueChange={onModel} disabled={disabled}>
        <SelectTrigger size="sm" className={`${TRIGGER} capitalize`} aria-label="Model">
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
    </>
  )
}
