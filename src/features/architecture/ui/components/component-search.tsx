import { useEffect } from "react"

import { CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command"
import type { SystemModel } from "@/shared/bindings/SystemModel"

import { useSystemUi } from "../../model/system-store"
import { featureId } from "../../model/system-view"
import { NodeIcon, ROLE_LABEL } from "./system-style"

interface ComponentSearchProps {
  model: SystemModel
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Only the visible tab listens for ⌘K. */
  listen: boolean
}

/** ⌘K: jump to any component; focuses its feature and selects it. */
export function ComponentSearch({ model, open, onOpenChange, listen }: ComponentSearchProps) {
  const ui = useSystemUi()
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

  return (
    <CommandDialog open={open} onOpenChange={onOpenChange} title="Find component" description="Jump to a component">
      <CommandInput placeholder="Find a component, channel or external system…" />
      <CommandList>
        <CommandEmpty>No components found.</CommandEmpty>
        <CommandGroup heading={`${model.nodes.length} components`}>
          {model.nodes.map((n) => (
            <CommandItem
              key={n.id}
              value={`${n.name} ${n.feature} ${n.role}`}
              onSelect={() => {
                if (n.kind === "component") ui.focus(featureId(n))
                ui.select(n.id)
                onOpenChange(false)
              }}
            >
              <NodeIcon node={n} className="size-4 text-muted-foreground" />
              <span className="font-medium">{n.name}</span>
              <span className="ml-auto truncate pl-4 text-xs text-muted-foreground">
                {n.feature} · {ROLE_LABEL[n.role]}
              </span>
            </CommandItem>
          ))}
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  )
}
