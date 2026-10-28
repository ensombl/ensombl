import { useMemo, useState, type ReactNode } from "react"
import { Boxes, ChevronRight, Code2, DoorOpen, Eye, EyeOff, RotateCcw, Split, X } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Textarea } from "@/components/ui/textarea"
import type { ComponentOverride } from "@/shared/bindings/ComponentOverride"
import type { Interaction } from "@/shared/bindings/Interaction"
import type { Layer } from "@/shared/bindings/Layer"
import type { Role } from "@/shared/bindings/Role"
import type { SystemNode } from "@/shared/bindings/SystemNode"
import type { SystemView } from "@/shared/bindings/SystemView"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { usePeekStore } from "@/features/peek/model/peek-store"

import { useArchitectureCommands } from "../../commands/architecture-commands"
import { LAYER_LABEL } from "../../layout/system-layout"
import { useSystemUi } from "../../model/system-store"
import { KIND_STYLE, LAYER_COLOR, NodeIcon, ROLE_LABEL } from "./system-style"

const ROLES = Object.keys(ROLE_LABEL).filter((r) => r !== "channel" && r !== "external") as Role[]
const LAYERS = Object.keys(LAYER_LABEL).filter((l) => l !== "external") as Layer[]
const MAX_MEMBERS = 8

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="space-y-2">
      <h3 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{title}</h3>
      {children}
    </div>
  )
}

function InteractionRow({
  interaction,
  other,
  onNavigate,
  onSite,
}: {
  interaction: Interaction
  other: SystemNode | undefined
  onNavigate: (id: string) => void
  onSite: (file: string, line: number) => void
}) {
  const style = KIND_STYLE[interaction.kind]
  return (
    <Collapsible className="rounded-md border">
      <div className="flex items-center gap-2 px-2 py-1.5">
        <span className="h-0.5 w-3 shrink-0 rounded" style={{ background: style.color }} title={style.label} />
        <button
          type="button"
          className="flex min-w-0 items-center gap-1.5 text-left text-sm hover:underline"
          onClick={() => other && onNavigate(other.id)}
        >
          {other && <NodeIcon node={other} className="size-3.5 shrink-0 text-muted-foreground" />}
          <span className="truncate font-medium">{other?.name ?? "?"}</span>
        </button>
        <CollapsibleTrigger className="group ml-auto flex shrink-0 items-center gap-1 text-xs text-muted-foreground hover:text-foreground">
          {interaction.count}×
          <ChevronRight className="size-3.5 transition-transform group-data-[state=open]:rotate-90" />
        </CollapsibleTrigger>
      </div>
      {interaction.members.length > 0 && (
        <div className="flex flex-wrap gap-1 px-2 pb-2">
          {interaction.members.slice(0, MAX_MEMBERS).map((m) => (
            <Badge key={m} variant="secondary" className="max-w-full truncate font-mono text-[10px] font-normal">
              {m}
            </Badge>
          ))}
          {interaction.members.length > MAX_MEMBERS && (
            <span className="text-[10px] text-muted-foreground">+{interaction.members.length - MAX_MEMBERS}</span>
          )}
        </div>
      )}
      <CollapsibleContent className="space-y-0.5 border-t px-2 py-1.5">
        {interaction.sites.map((s, i) => (
          <button
            key={i}
            type="button"
            onClick={() => onSite(s.file, s.line)}
            className="block w-full truncate text-left font-mono text-[11px] text-muted-foreground hover:text-foreground"
          >
            {s.file}:{s.line}
          </button>
        ))}
      </CollapsibleContent>
    </Collapsible>
  )
}

interface ComponentInspectorProps {
  system: SystemView
  node: SystemNode
  onNavigate: (id: string) => void
  onShowClasses: (symbolId: string) => void
  onClose: () => void
}

/** Details of one component; edits are saved to `.crm/architecture.json`. */
export function ComponentInspector({ system, node, onNavigate, onShowClasses, onClose }: ComponentInspectorProps) {
  const worktree = useWorkspace()
  const openPeek = usePeekStore((s) => s.open)
  const ui = useSystemUi()
  const { update, split } = useArchitectureCommands(worktree.id)
  const override = system.overrides.components[node.id] ?? { hidden: false }
  const [name, setName] = useState(node.name)
  const [description, setDescription] = useState(node.description ?? "")
  const [feature, setFeature] = useState(node.feature)
  const editable = node.kind === "component"

  const save = (patch: Partial<ComponentOverride>) =>
    update.mutate({ componentId: node.id, value: { ...override, hidden: override.hidden ?? false, ...patch } })

  const byId = useMemo(() => new Map(system.model.nodes.map((n) => [n.id, n])), [system.model.nodes])
  const outgoing = system.model.interactions.filter((i) => i.from === node.id)
  const incoming = system.model.interactions.filter((i) => i.to === node.id)
  const entries = system.model.entryPoints.filter((e) => e.node === node.id)
  const peek = (file: string, line: number) => openPeek(worktree.id, { file, line, title: file.split("/").pop() ?? file })

  return (
    <aside className="@container absolute inset-x-3 bottom-3 z-20 flex max-h-[48%] flex-col overflow-hidden rounded-xl border bg-popover text-popover-foreground shadow-lg">
      <header className="flex items-center gap-2 border-b p-2 pl-3">
        <NodeIcon node={node} className="size-4 shrink-0" />
        {editable ? (
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            onBlur={() => name.trim() && name !== node.name && save({ name: name.trim() })}
            onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
            className="h-7 border-transparent px-1 text-sm font-semibold shadow-none hover:border-input dark:bg-transparent"
            aria-label="Component name"
          />
        ) : (
          <span className="truncate text-sm font-semibold">{node.name}</span>
        )}
        <Button variant="ghost" size="icon" className="ml-auto size-7 shrink-0" onClick={onClose} aria-label="Close">
          <X />
        </Button>
      </header>
      {/* A plain overflow box: a ScrollArea can't size itself inside a max-height panel. */}
      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="grid gap-5 p-3 @2xl:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)]">
          <div className="min-w-0 space-y-4">
            {editable && (
              <div className="grid grid-cols-2 gap-2">
                <Select value={node.role} onValueChange={(role) => save({ role: role as Role })}>
                  <SelectTrigger size="sm" className="h-8 w-full text-xs" aria-label="Role">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ROLES.map((r) => (
                      <SelectItem key={r} value={r} className="text-xs">
                        {ROLE_LABEL[r]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Select value={node.layer} onValueChange={(layer) => save({ layer: layer as Layer })}>
                  <SelectTrigger size="sm" className="h-8 w-full text-xs" aria-label="Layer">
                    <span className="size-2 rounded-full" style={{ background: LAYER_COLOR[node.layer] }} />
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {LAYERS.map((l) => (
                      <SelectItem key={l} value={l} className="text-xs">
                        {LAYER_LABEL[l]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Input
                  value={feature}
                  onChange={(e) => setFeature(e.target.value)}
                  onBlur={() => feature.trim() && feature !== node.feature && save({ feature: feature.trim() })}
                  onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
                  className="col-span-2 h-8 text-xs"
                  aria-label="Feature"
                  placeholder="Feature"
                />
              </div>
            )}

            <Section title="Description">
              <Textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                onBlur={() => description !== (node.description ?? "") && save({ description: description.trim() || undefined })}
                placeholder="What is this responsible for? Agents see this when you tag it."
                className="min-h-20 text-sm"
              />
            </Section>

            <div className="flex flex-wrap gap-1.5">
              {node.file && (
                <Button size="sm" variant="outline" className="h-7 text-xs" onClick={() => ui.showFile(node.file!, node.line ?? 1)}>
                  <Code2 /> Show code
                </Button>
              )}
              {node.symbolIds.length > 0 && (
                <Button size="sm" variant="outline" className="h-7 text-xs" onClick={() => onShowClasses(node.symbolIds[0])}>
                  <Boxes /> Show classes
                </Button>
              )}
              {editable && (
                <Button size="sm" variant="outline" className="h-7 text-xs" onClick={() => save({ hidden: !node.hidden })}>
                  {node.hidden ? <Eye /> : <EyeOff />} {node.hidden ? "Show" : "Hide"}
                </Button>
              )}
              {node.merged.length > 0 && (
                <Button size="sm" variant="outline" className="h-7 text-xs" onClick={() => split.mutate(node.id, { onSuccess: onClose })}>
                  <Split /> Split
                </Button>
              )}
              {system.overrides.components[node.id] && (
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 text-xs"
                  onClick={() => update.mutate({ componentId: node.id, value: { hidden: false } })}
                >
                  <RotateCcw /> Reset
                </Button>
              )}
          </div>

          {node.file && (
            <button
              type="button"
              onClick={() => peek(node.file!, node.line ?? 1)}
              className="block w-full truncate text-left font-mono text-[11px] text-muted-foreground hover:text-foreground"
              title={node.file}
            >
              {node.file}
              {node.line ? `:${node.line}` : ""}
            </button>
          )}
          </div>

          <div className="min-w-0 space-y-4">
            {node.merged.length > 0 && (
              <Section title={`Merged from ${node.merged.length}`}>
                <p className="text-xs text-muted-foreground">{node.merged.map((id) => id.split("#").pop()).join(", ")}</p>
              </Section>
            )}

            {entries.length > 0 && (
              <Section title={`Entry points (${entries.length})`}>
                <div className="space-y-1">
                  {entries.map((e) => (
                    <button
                      key={e.id}
                      type="button"
                      onClick={() => ui.openEntry(e.id)}
                      className="flex w-full items-center gap-2 rounded-md px-1.5 py-1 text-left text-xs hover:bg-accent"
                    >
                      <DoorOpen className="size-3.5 shrink-0 text-muted-foreground" />
                      <span className="truncate font-mono">{e.label}</span>
                    </button>
                  ))}
                </div>
              </Section>
            )}

            {outgoing.length > 0 && (
              <Section title={`Talks to (${outgoing.length})`}>
                <div className="space-y-1.5">
                  {outgoing.map((i) => (
                    <InteractionRow key={i.id} interaction={i} other={byId.get(i.to)} onNavigate={onNavigate} onSite={peek} />
                  ))}
                </div>
              </Section>
            )}
            {incoming.length > 0 && (
              <Section title={`Used by (${incoming.length})`}>
                <div className="space-y-1.5">
                  {incoming.map((i) => (
                    <InteractionRow key={i.id} interaction={i} other={byId.get(i.from)} onNavigate={onNavigate} onSite={peek} />
                  ))}
                </div>
              </Section>
            )}

            {entries.length === 0 && outgoing.length === 0 && incoming.length === 0 && (
              <p className="text-xs text-muted-foreground">No interactions found for this component.</p>
            )}
          </div>
        </div>
      </div>
    </aside>
  )
}
