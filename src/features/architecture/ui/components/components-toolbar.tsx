import { ChevronLeft, LayoutGrid, Search, SlidersHorizontal } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import type { InteractionKind } from "@/shared/bindings/InteractionKind"
import type { SystemModel } from "@/shared/bindings/SystemModel"

import { useSystemUi } from "../../model/system-store"
import { ALL_KINDS, type ComponentsView, type SystemFilters } from "../../model/system-view"
import { KIND_STYLE } from "./system-style"

const TOGGLES: { key: keyof Omit<SystemFilters, "kinds">; label: string }[] = [
  { key: "showExternal", label: "External systems" },
  { key: "showChannels", label: "Event channels" },
  { key: "showUnclassified", label: "Unclassified classes" },
  { key: "showHidden", label: "Hidden components" },
]

interface ComponentsToolbarProps {
  model: SystemModel
  view: ComponentsView
  onSearch: () => void
}

export function ComponentsToolbar({ model, view, onSearch }: ComponentsToolbarProps) {
  const ui = useSystemUi()
  const { focus, filters } = ui.state
  const focused = focus && focus !== "all" ? focus.slice("feature:".length) : null
  const components = model.nodes.filter((n) => n.kind === "component").length

  return (
    <div className="flex flex-wrap items-center gap-2 rounded-lg border bg-background/90 p-1.5 shadow-sm backdrop-blur">
      {focus !== null && (
        <Button variant="ghost" size="sm" className="h-8 gap-1 px-2 text-xs" onClick={() => ui.focus(null)}>
          <ChevronLeft /> All features
        </Button>
      )}
      <span className="px-1 text-sm font-medium">
        {focused ? focused.split("/").pop() : view.overview ? "Features" : "All components"}
      </span>
      {view.overview && (
        <span className="text-xs text-muted-foreground">Double-click a feature to open it</span>
      )}
      <Button variant="outline" size="sm" className="h-8 gap-2 text-muted-foreground" onClick={onSearch}>
        <Search /> Find
        <kbd className="rounded border bg-muted px-1 font-mono text-[10px]">⌘K</kbd>
      </Button>
      {focus !== "all" && (
        <Button variant="outline" size="sm" className="h-8 gap-1.5 text-xs" onClick={() => ui.focus("all")} title={`${components} components`}>
          <LayoutGrid /> Show everything
        </Button>
      )}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className="h-8 gap-1.5 text-xs">
            <SlidersHorizontal /> View
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-56">
          <DropdownMenuLabel>Show</DropdownMenuLabel>
          {TOGGLES.map((t) => (
            <DropdownMenuCheckboxItem
              key={t.key}
              checked={filters[t.key]}
              onCheckedChange={(checked) => ui.setFilters({ [t.key]: checked })}
              onSelect={(e) => e.preventDefault()}
            >
              {t.label}
            </DropdownMenuCheckboxItem>
          ))}
          <DropdownMenuSeparator />
          <DropdownMenuLabel>Interactions</DropdownMenuLabel>
          {ALL_KINDS.map((kind) => (
            <DropdownMenuCheckboxItem
              key={kind}
              checked={filters.kinds.includes(kind)}
              onCheckedChange={(checked) =>
                ui.setFilters({ kinds: checked ? [...filters.kinds, kind] : filters.kinds.filter((k: InteractionKind) => k !== kind) })
              }
              onSelect={(e) => e.preventDefault()}
            >
              <span className="mr-1 inline-block h-0.5 w-4 rounded" style={{ background: KIND_STYLE[kind].color }} />
              {KIND_STYLE[kind].label}
            </DropdownMenuCheckboxItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  )
}
