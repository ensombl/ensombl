import { useEffect, useMemo } from "react"

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import type { ArchModel } from "@/shared/bindings/ArchModel"

interface SymbolSearchProps {
  model: ArchModel
  open: boolean
  onOpenChange: (open: boolean) => void
  onSelect: (symbolId: string) => void
  /** Only the visible tab listens for ⌘K. */
  listen: boolean
}

const LIMIT = 4000

export function SymbolSearch({ model, open, onOpenChange, onSelect, listen }: SymbolSearchProps) {
  useEffect(() => {
    if (!listen) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault()
        onOpenChange(!open)
      }
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [listen, open, onOpenChange])

  const symbols = useMemo(() => model.symbols.slice(0, LIMIT), [model])

  return (
    <CommandDialog open={open} onOpenChange={onOpenChange} title="Find symbol" description="Jump to a class, interface or function">
      <CommandInput placeholder="Find a class, interface, function…" />
      <CommandList>
        <CommandEmpty>No symbols found.</CommandEmpty>
        <CommandGroup heading={`${model.symbols.length} symbols`}>
          {symbols.map((s) => (
            <CommandItem
              key={s.id}
              value={`${s.name} ${s.file}`}
              onSelect={() => {
                onSelect(s.id)
                onOpenChange(false)
              }}
            >
              <span className="font-medium">{s.name}</span>
              <span className="ml-auto truncate pl-4 font-mono text-xs text-muted-foreground">{s.file}</span>
            </CommandItem>
          ))}
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  )
}
