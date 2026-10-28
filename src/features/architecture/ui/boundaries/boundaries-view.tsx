import { useMemo } from "react"
import { ArrowRight, ShieldCheck, TriangleAlert } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Switch } from "@/components/ui/switch"
import { cn } from "@/lib/utils"
import type { SystemView } from "@/shared/bindings/SystemView"
import { useWorkspace } from "@/shared/workspace/workspace-context"

import { useArchitectureCommands } from "../../commands/architecture-commands"
import { LAYER_LABEL } from "../../layout/system-layout"
import { featureLayers, featureMatrix, LAYERS } from "../../model/boundaries"
import { useSystemUi } from "../../model/system-store"
import { featureId } from "../../model/system-view"
import { LAYER_COLOR, NodeIcon } from "../components/system-style"

function Heading({ title, hint }: { title: string; hint: string }) {
  return (
    <div className="space-y-0.5">
      <h3 className="text-sm font-medium">{title}</h3>
      <p className="text-xs text-muted-foreground">{hint}</p>
    </div>
  )
}

/** Features and layers at a glance, who depends on whom, and what breaks the rules. */
export function BoundariesView({ system }: { system: SystemView }) {
  const worktree = useWorkspace()
  const ui = useSystemUi()
  const { setRule } = useArchitectureCommands(worktree.id)
  const { model } = system
  const rows = useMemo(() => featureLayers(model), [model])
  const matrix = useMemo(() => featureMatrix(model), [model])
  const nodes = useMemo(() => new Map(model.nodes.map((n) => [n.id, n])), [model.nodes])
  const interactions = useMemo(() => new Map(model.interactions.map((i) => [i.id, i])), [model.interactions])
  const maxLayer = Math.max(1, ...rows.flatMap((r) => LAYERS.map((l) => r.byLayer[l])))

  const openFeature = (id: string) => {
    ui.focus(id)
    ui.setTab("components")
  }
  const openNode = (id: string) => {
    const n = nodes.get(id)
    if (!n) return
    ui.focus(featureId(n))
    ui.select(id)
    ui.setTab("components")
  }

  return (
    <div className="flex h-full">
      <ScrollArea className="min-w-0 flex-1">
        <div className="space-y-8 p-4">
          <section className="space-y-3">
            <Heading title="Layers by feature" hint="Components per layer. Click a feature to open it on the map." />
            <div className="overflow-x-auto rounded-md border">
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b bg-muted/40">
                    <th className="px-3 py-2 text-left font-medium">Feature</th>
                    {LAYERS.map((l) => (
                      <th key={l} className="px-3 py-2 text-right font-medium" style={{ color: LAYER_COLOR[l] }}>
                        {LAYER_LABEL[l]}
                      </th>
                    ))}
                    <th className="px-3 py-2 text-right font-medium">Total</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r) => (
                    <tr key={r.id} className="cursor-default border-b last:border-0 hover:bg-accent/40" onClick={() => openFeature(r.id)}>
                      <td className="px-3 py-1.5">
                        <span className="font-medium">{r.label}</span>
                        <span className="ml-2 font-mono text-[10px] text-muted-foreground">{r.pkg}</span>
                      </td>
                      {LAYERS.map((l) => (
                        <td key={l} className="px-3 py-1.5 text-right font-mono">
                          {r.byLayer[l] > 0 ? (
                            <span
                              className="inline-block min-w-6 rounded px-1"
                              style={{ background: LAYER_COLOR[l].replace(")", ` / ${0.1 + (0.4 * r.byLayer[l]) / maxLayer})`) }}
                            >
                              {r.byLayer[l]}
                            </span>
                          ) : (
                            <span className="text-muted-foreground/40">·</span>
                          )}
                        </td>
                      ))}
                      <td className="px-3 py-1.5 text-right font-mono text-muted-foreground">{r.total}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          {matrix.features.length > 1 && (
            <section className="space-y-3">
              <Heading
                title="Feature dependencies"
                hint={`Calls, injections and requests from the feature on the left to the one on top. Most connected ${matrix.features.length} features.`}
              />
              <div className="overflow-auto rounded-md border p-2">
                <table className="text-[11px]">
                  <thead>
                    <tr>
                      <th />
                      {matrix.features.map((f) => (
                        <th key={f.id} className="h-28 w-7 align-bottom font-normal">
                          <div className="w-7 origin-bottom-left translate-x-5 -rotate-45 truncate text-left whitespace-nowrap text-muted-foreground">
                            {f.label}
                          </div>
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {matrix.features.map((from, r) => (
                      <tr key={from.id}>
                        <th className="max-w-40 truncate pr-2 text-right font-normal text-muted-foreground">{from.label}</th>
                        {matrix.cells[r].map((n, c) => (
                          <td key={c} className="p-0.5">
                            <div
                              title={n ? `${from.label} → ${matrix.features[c].label}: ${n}` : undefined}
                              className={cn("flex size-6 items-center justify-center rounded-sm font-mono text-[9px]", r === c && "bg-muted/40")}
                              style={n ? { background: `oklch(0.72 0.12 250 / ${0.15 + (0.75 * n) / matrix.max})` } : undefined}
                            >
                              {n > 0 ? n : ""}
                            </div>
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          )}
        </div>
      </ScrollArea>

      <aside className="flex w-96 shrink-0 flex-col border-l">
        <div className="space-y-3 border-b p-3">
          <Heading title="Rules" hint="Saved to .crm/architecture.json." />
          {system.rules.map((rule) => (
            <label key={rule.id} className="flex items-center gap-2 text-xs">
              <Switch checked={rule.enabled} onCheckedChange={(enabled) => setRule.mutate({ rule: rule.id, enabled })} />
              <span className={cn(!rule.enabled && "text-muted-foreground line-through")}>{rule.message}</span>
              <Badge variant="secondary" className="ml-auto font-mono text-[10px] font-normal">
                {system.violations.filter((v) => v.rule === rule.id).length}
              </Badge>
            </label>
          ))}
        </div>
        <ScrollArea className="min-h-0 flex-1">
          <div className="space-y-1.5 p-3">
            {system.violations.length === 0 ? (
              <p className="flex items-center gap-2 text-sm text-muted-foreground">
                <ShieldCheck className="size-4" /> No violations.
              </p>
            ) : (
              system.violations.map((v) => {
                const i = interactions.get(v.interaction)
                const from = i && nodes.get(i.from)
                const to = i && nodes.get(i.to)
                if (!i || !from || !to) return null
                return (
                  <button
                    key={v.interaction + v.rule}
                    type="button"
                    onClick={() => openNode(from.id)}
                    className="w-full space-y-1 rounded-md border p-2 text-left hover:bg-accent/40"
                  >
                    <p className="flex items-center gap-1.5 text-[11px] text-amber-300">
                      <TriangleAlert className="size-3.5" /> {v.message}
                    </p>
                    <p className="flex min-w-0 items-center gap-1.5 text-xs">
                      <NodeIcon node={from} className="size-3.5 shrink-0 text-muted-foreground" />
                      <span className="truncate font-medium">{from.name}</span>
                      <ArrowRight className="size-3 shrink-0 text-muted-foreground" />
                      <NodeIcon node={to} className="size-3.5 shrink-0 text-muted-foreground" />
                      <span className="truncate font-medium">{to.name}</span>
                    </p>
                    {i.members.length > 0 && (
                      <p className="truncate font-mono text-[10px] text-muted-foreground">{i.members.slice(0, 4).join(", ")}</p>
                    )}
                  </button>
                )
              })
            )}
          </div>
        </ScrollArea>
      </aside>
    </div>
  )
}
