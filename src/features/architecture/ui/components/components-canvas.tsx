import { useEffect, useMemo, useRef, useState } from "react"
import {
  Background,
  BackgroundVariant,
  Controls,
  Panel,
  PanOnScrollMode,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Node,
} from "@xyflow/react"
import { Combine, EyeOff } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import type { SystemView } from "@/shared/bindings/SystemView"
import { useIsActiveWorkspace, useWorkspace } from "@/shared/workspace/workspace-context"

import { useArchitectureCommands } from "../../commands/architecture-commands"
import { runLayout } from "../../layout/run-layout"
import { flattenLayout, lanesOf, systemLayoutSignature, toSystemElk, type Layout } from "../../layout/system-layout"
import { useChanges } from "../../model/changes-store"
import { useSystemUi } from "../../model/system-store"
import { buildComponentsView, featureId } from "../../model/system-view"
import { ComponentInspector } from "./component-inspector"
import { ComponentSearch } from "./component-search"
import { ComponentsToolbar } from "./components-toolbar"
import { SystemEdgeView, type SystemFlowEdge } from "./system-edge"
import {
  ComponentNodeView,
  FeatureNodeView,
  LaneNodeView,
  PackageNodeView,
  type ComponentFlowNode,
  type FeatureFlowNode,
  type LaneFlowNode,
  type PackageFlowNode,
} from "./system-nodes"
import { SystemMarkers } from "./system-style"
import { sideHandles } from "../../layout/handles"

const nodeTypes = { component: ComponentNodeView, feature: FeatureNodeView, lane: LaneNodeView, package: PackageNodeView }
const edgeTypes = { system: SystemEdgeView }

/** Edge labels are drawn for every edge only when the view is this small. */
const LABEL_ALL_EDGES = 40
/** Viewport culling kicks in above this many nodes. */
const CULL_ABOVE = 150

interface CanvasProps {
  system: SystemView
  onShowCode: (symbolId: string) => void
}

function Canvas({ system, onShowCode }: CanvasProps) {
  const { model } = system
  const worktree = useWorkspace()
  const active = useIsActiveWorkspace()
  const ui = useSystemUi()
  const { focus, selected, filters } = ui.state
  const flow = useReactFlow()
  const commands = useArchitectureCommands(worktree.id)
  const [layout, setLayout] = useState<Layout | null>(null)
  const [multi, setMulti] = useState<string[]>([])
  const [mergeName, setMergeName] = useState("")
  const [searchOpen, setSearchOpen] = useState(false)
  // Remembers which view the hover happened in, so it can't outlive that view.
  const [hover, setHover] = useState<{ id: string; view: unknown } | null>(null)


  const view = useMemo(() => buildComponentsView(model, focus, filters), [model, focus, filters])
  // Hovering previews a node's connections; selection wins. Only nodes in the
  // view count: a stale id (left without a mouseleave) would dim every edge.
  const inView = (id: string | null) => (id && view.nodes.some((n) => n.id === id) ? id : null)
  const hovered = hover?.view === view ? hover.id : null
  const focusNode = inView(selected) ?? inView(hovered)
  const signature = useMemo(() => systemLayoutSignature(view), [view])
  const latest = useRef(view)
  useEffect(() => {
    latest.current = view
  })

  useEffect(() => {
    let cancelled = false
    runLayout(toSystemElk(latest.current))
      .then((result) => {
        if (cancelled) return
        setLayout(flattenLayout(result, latest.current.grouped))
        requestAnimationFrame(() => flow.fitView({ padding: 0.1, maxZoom: 1, duration: 300 }))
      })
      .catch((error) => console.error("layout failed", error))
    return () => {
      cancelled = true
    }
  }, [signature, flow])

  // Components (and their features) whose code changed on disk.
  const changes = useChanges()
  const changedAt = useMemo(() => {
    const at = new Map<string, number>()
    for (const n of model.nodes) {
      const time = Math.max(0, ...n.symbolIds.map((id) => changes.changed[id] ?? 0))
      if (!time) continue
      at.set(n.id, time)
      if (n.kind === "component") at.set(featureId(n), Math.max(at.get(featureId(n)) ?? 0, time))
    }
    return at
  }, [model, changes.changed])

  // Highlight what the selected node talks to.
  const neighbors = useMemo(() => {
    if (!focusNode) return null
    const set = new Set([focusNode])
    for (const e of view.edges) {
      if (e.source === focusNode) set.add(e.target)
      if (e.target === focusNode) set.add(e.source)
    }
    return set
  }, [view, focusNode])

  const nodes = useMemo<Node[]>(() => {
    if (!layout) return []
    const lanes: LaneFlowNode[] = lanesOf(view, layout).map((l) => ({
      id: `lane:${l.layer}`,
      type: "lane",
      position: { x: l.x, y: l.y },
      width: l.width,
      height: l.height,
      data: { layer: l.layer },
      selectable: false,
      draggable: false,
      zIndex: -1,
      // Decoration only: clicks fall through to the pane.
      style: { pointerEvents: "none" },
    }))
    const items = view.nodes.flatMap((n): Node[] => {
      const box = layout.nodes.get(n.id)
      if (!box) return []
      if (n.type === "package") {
        // Decoration only: clicks fall through to the pane or the nodes inside.
        const node: PackageFlowNode = {
          id: n.id,
          type: "package",
          position: { x: box.x, y: box.y },
          width: box.width,
          height: box.height,
          data: { label: n.label, count: n.count },
          selectable: false,
          zIndex: -1,
          style: { pointerEvents: "none" },
        }
        return [node]
      }
      const base = {
        id: n.id,
        position: { x: box.x, y: box.y },
        width: box.width,
        height: box.height,
        handles: sideHandles(box.width, box.height),
        parentId: n.parent && layout.nodes.has(n.parent) ? n.parent : undefined,
        selected: n.id === selected || multi.includes(n.id),
      }
      const dimmed = !!neighbors && !neighbors.has(n.id)
      if (n.type === "feature") {
        const node: FeatureFlowNode = {
          ...base,
          type: "feature",
          data: { label: n.label, pkg: n.pkg, count: n.count, entryCount: n.entryCount, roles: n.roles, dimmed, changedAt: changedAt.get(n.id) },
        }
        return [node]
      }
      const node: ComponentFlowNode = {
        ...base,
        type: "component",
        data: { node: n.node, entryCount: n.entryCount, context: n.context, dimmed, changedAt: changedAt.get(n.id) },
      }
      return [node]
    })
    return [...lanes, ...items]
  }, [view, layout, selected, multi, neighbors, changedAt])

  const edges = useMemo<SystemFlowEdge[]>(() => {
    if (!layout) return []
    const showLabel = view.edges.length <= LABEL_ALL_EDGES
    const busy = !showLabel
    return view.edges
      .filter((e) => layout.nodes.has(e.source) && layout.nodes.has(e.target))
      .map((e) => {
        const highlighted = e.source === focusNode || e.target === focusNode
        return {
          id: e.id,
          source: e.source,
          target: e.target,
          type: "system",
          zIndex: highlighted ? 10 : 1,
          data: {
            kind: e.kind,
            members: e.members,
            count: e.count,
            highlighted,
            dimmed: !!focusNode && !highlighted,
            showLabel,
            busy,
            points: layout.routes.get(e.id),
          },
        }
      })
  }, [view, layout, focusNode])

  const selectedNode = selected ? model.nodes.find((n) => n.id === selected) : undefined
  const mergeable = multi.filter((id) => model.nodes.some((n) => n.id === id && n.kind === "component"))

  return (
    <div className="flex h-full">
      <div className="relative min-w-0 flex-1">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          colorMode="dark"
          minZoom={0.05}
          maxZoom={2}
          nodesConnectable={false}
          nodesDraggable={false}
          panOnScroll
          panOnScrollMode={PanOnScrollMode.Free}
          zoomOnScroll={false}
          zoomOnPinch
          zoomOnDoubleClick={false}
          multiSelectionKeyCode="Shift"
          onSelectionChange={({ nodes: picked }) => {
            const ids = picked.length > 1 ? picked.map((n) => n.id) : []
            // Keep the same array when nothing changed, or this feeds back into the nodes forever.
            setMulti((prev) => (prev.length === ids.length && prev.every((id, i) => id === ids[i]) ? prev : ids))
          }}
          onNodeClick={(event, node) => {
            if (event.shiftKey || node.type === "lane" || node.type === "package") return
            if (node.type === "feature") {
              ui.select(null)
              return
            }
            ui.select(node.id)
            const picked = model.nodes.find((n) => n.id === node.id)
            if (picked) changes.ack(picked.symbolIds)
          }}
          onNodeMouseEnter={(_, node) => node.type !== "lane" && node.type !== "package" && setHover({ id: node.id, view })}
          onNodeMouseLeave={() => setHover(null)}
          onNodeDoubleClick={(_, node) => {
            if (node.type === "feature") ui.focus(node.id)
          }}
          onPaneClick={() => {
            ui.select(null)
            setMulti((prev) => (prev.length ? [] : prev))
          }}
          proOptions={{ hideAttribution: true }}
          // Culling can drop edges whose ends are offscreen; only worth it on huge views.
          onlyRenderVisibleElements={view.nodes.length > CULL_ABOVE}
        >
          <SystemMarkers />
          <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
          <Panel position="top-left">
            <ComponentsToolbar model={model} view={view} onSearch={() => setSearchOpen(true)} />
          </Panel>
          {mergeable.length > 1 && (
            <Panel position="bottom-center">
              <div className="flex items-center gap-2 rounded-lg border bg-popover p-1.5 pl-3 text-sm shadow-md">
                <span className="text-muted-foreground">{mergeable.length} selected</span>
                <Popover>
                  <PopoverTrigger asChild>
                    <Button size="sm" className="h-7">
                      <Combine /> Merge
                    </Button>
                  </PopoverTrigger>
                  <PopoverContent className="w-64 space-y-2" align="center">
                    <p className="text-sm font-medium">Merge into one component</p>
                    <form
                      className="flex gap-2"
                      onSubmit={(e) => {
                        e.preventDefault()
                        commands.merge.mutate(
                          { componentIds: mergeable, name: mergeName },
                          {
                            onSuccess: (id) => {
                              setMulti([])
                              setMergeName("")
                              ui.select(id)
                            },
                          },
                        )
                      }}
                    >
                      <Input autoFocus placeholder="Name" value={mergeName} onChange={(e) => setMergeName(e.target.value)} className="h-8" />
                      <Button type="submit" size="sm" className="h-8" disabled={!mergeName.trim()}>
                        Merge
                      </Button>
                    </form>
                  </PopoverContent>
                </Popover>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7"
                  onClick={() => {
                    for (const id of mergeable) {
                      commands.update.mutate({ componentId: id, value: { ...system.overrides.components[id], hidden: true } })
                    }
                    setMulti([])
                  }}
                >
                  <EyeOff /> Hide
                </Button>
              </div>
            </Panel>
          )}
          {!selectedNode && <Controls showInteractive={false} position="bottom-left" />}
        </ReactFlow>
        {selectedNode && (
          <ComponentInspector
            key={selectedNode.id}
            system={system}
            node={selectedNode}
            onNavigate={(id) => {
              const target = model.nodes.find((n) => n.id === id)
              if (target && focus !== null && focus !== "all" && !view.nodes.some((n) => n.id === id)) {
                ui.focus(`feature:${target.package}/${target.feature}`)
              }
              ui.select(id)
            }}
            onShowCode={onShowCode}
            onClose={() => ui.select(null)}
          />
        )}
      </div>
      <ComponentSearch model={model} open={searchOpen} onOpenChange={setSearchOpen} listen={active && ui.state.tab === "components"} />
    </div>
  )
}

export function ComponentsCanvas(props: CanvasProps) {
  return (
    <ReactFlowProvider>
      <Canvas {...props} />
    </ReactFlowProvider>
  )
}
