import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  Panel,
  PanOnScrollMode,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Node,
} from "@xyflow/react"

import type { ArchModel } from "@/shared/bindings/ArchModel"
import { useIsActiveWorkspace } from "@/shared/workspace/workspace-context"

import { flattenLayout, layoutSignature, toElkGraph, type Layout } from "../../layout/elk-graph"
import { verticalHandles } from "../../layout/handles"
import { layoutCache } from "../../layout/layout-cache"
import { runLayout } from "../../layout/run-layout"
import { useChanges } from "../../model/changes-store"
import { useDiagram } from "../../model/diagram-store"
import { useSystemUi } from "../../model/system-store"
import { packageColor } from "../../model/package-colors"
import { ancestorsOf, buildHierarchy, buildView, defaultExpanded, expandOneLevel } from "../../model/view"
import { PatternsPanel } from "@/features/patterns/ui/patterns-panel"

import { SymbolInspector } from "../inspector/symbol-inspector"
import { DiagramToolbar } from "./diagram-toolbar"
import { GroupNodeComponent, type GroupFlowNode } from "./group-node"
import { SymbolNodeComponent, type SymbolFlowNode } from "./symbol-node"
import { SymbolSearch } from "./symbol-search"
import { UmlEdgeComponent, UmlMarkers, type UmlFlowEdge } from "./uml-edge"

const nodeTypes = { symbol: SymbolNodeComponent, group: GroupNodeComponent }
const edgeTypes = { uml: UmlEdgeComponent }
const MINIMAP_MAX_NODES = 300
/** Viewport culling mounts and unmounts nodes while panning; only worth it on huge views. */
const CULL_ABOVE = 1000
const layouts = layoutCache<{ layout: Layout; routed: Set<string> }>()

function Canvas({ model }: { model: ArchModel }) {
  const diagram = useDiagram()
  const { state, init, clearFocus, select } = diagram
  const workspaceActive = useIsActiveWorkspace()
  const tab = useSystemUi().state.tab
  // ⌘K belongs to whichever lens is showing.
  const active = workspaceActive && tab === "code"
  const flow = useReactFlow()
  const [computed, setLayout] = useState<{ layout: Layout; routed: Set<string> } | null>(null)
  const [searchOpen, setSearchOpen] = useState(false)
  const fitted = useRef(false)

  useEffect(() => {
    if (!state.initialized) init(defaultExpanded(model, state.filters))
  }, [state.initialized, state.filters, init, model])

  // Layout ignores selection so clicking around never moves nodes.
  const layoutView = useMemo(
    () => buildView(model, { expanded: state.expanded, filters: state.filters }),
    [model, state.expanded, state.filters],
  )
  const selected = state.selected
  const view = useMemo(
    () => (selected ? buildView(model, { expanded: state.expanded, filters: state.filters, selected }) : layoutView),
    [model, state.expanded, state.filters, selected, layoutView],
  )

  // Only re-run ELK when the structure changed, so live edits don't shuffle the map.
  const signature = useMemo(() => layoutSignature(layoutView), [layoutView])
  const latestView = useRef(layoutView)
  useEffect(() => {
    latestView.current = layoutView
  })
  // A cached layout for this structure is used right away; otherwise ELK runs.
  const layout = layouts.get(signature) ?? computed
  useEffect(() => {
    let cancelled = false
    if (layouts.get(signature)) return
    const { graph, routed } = toElkGraph(latestView.current)
    runLayout(graph)
      .then((result) => {
        if (cancelled) return
        const next = { layout: flattenLayout(result), routed }
        layouts.set(signature, next)
        setLayout(next)
      })
      .catch((error) => console.error("layout failed", error))
    return () => {
      cancelled = true
    }
  }, [signature])

  // Changed symbols mark themselves and every group that contains them.
  const changes = useChanges()
  const changedAt = useMemo(() => {
    const at = new Map<string, number>()
    const { ancestors } = buildHierarchy(model.symbols)
    for (const [id, time] of Object.entries(changes.changed)) {
      for (const key of [id, ...(ancestors.get(id) ?? [])]) at.set(key, Math.max(at.get(key) ?? 0, time))
    }
    return at
  }, [model, changes.changed])

  const spotlight = state.spotlight
  const nodes = useMemo<Node[]>(() => {
    if (!layout) return []
    const boxes = layout.layout.nodes
    return view.nodes.flatMap((n): Node[] => {
      const box = boxes.get(n.id)
      if (!box) return []
      const base = {
        id: n.id,
        position: { x: box.x, y: box.y },
        width: box.width,
        height: box.height,
        handles: verticalHandles(box.width, box.height),
        parentId: n.parent && boxes.has(n.parent) ? n.parent : undefined,
        selected: n.id === selected,
      }
      if (n.type === "group") {
        const node: GroupFlowNode = {
          ...base,
          type: "group",
          data: {
            label: n.label,
            title: n.title,
            pkg: n.pkg,
            level: n.level,
            expanded: n.expanded,
            symbolCount: n.symbolCount,
            changedAt: changedAt.get(n.id),
          },
          zIndex: n.expanded ? -1 : 0,
          selectable: !n.expanded,
        }
        return [node]
      }
      const role = spotlight?.roles[n.id]
      const node: SymbolFlowNode = {
        ...base,
        type: "symbol",
        data: { symbol: n.symbol, pkg: n.pkg, changedAt: changedAt.get(n.id), role, dimmed: !!spotlight && !role },
      }
      return [node]
    })
  }, [view, layout, selected, changedAt, spotlight])

  const edges = useMemo<UmlFlowEdge[]>(() => {
    if (!layout) return []
    const { nodes: boxes, routes } = layout.layout
    return view.edges
      .filter((e) => boxes.has(e.source) && boxes.has(e.target))
      .map((e) => {
        const inSpotlight = !!spotlight?.roles[e.source] && !!spotlight?.roles[e.target]
        const touches = e.source === selected || e.target === selected || inSpotlight
        return {
          id: e.id,
          source: e.source,
          target: e.target,
          type: "uml",
          zIndex: 0,
          data: {
            kind: e.kind,
            count: e.count,
            highlighted: touches,
            dimmed: (selected !== null || !!spotlight) && !touches,
            points: layout.routed.has(e.id) ? routes.get(e.id) : undefined,
          },
        }
      })
  }, [view, layout, selected, spotlight])

  // First layout: fit everything. Later: center on a revealed symbol.
  useEffect(() => {
    if (!layout) return
    if (!fitted.current) {
      fitted.current = true
      requestAnimationFrame(() => flow.fitView({ padding: 0.12, maxZoom: 1 }))
    }
    const focus = state.focus?.filter((id) => layout.layout.nodes.has(id))
    if (focus?.length) {
      clearFocus()
      const padding = focus.length === 1 ? 0.6 : 0.2
      requestAnimationFrame(() => flow.fitView({ nodes: focus.map((id) => ({ id })), duration: 500, maxZoom: 1.2, padding }))
    }
  }, [layout, flow, state.focus, clearFocus])

  const reveal = useCallback(
    (symbolId: string) => {
      const kind = model.symbols.find((s) => s.id === symbolId)?.kind
      diagram.reveal(symbolId, ancestorsOf(model, symbolId), kind === "function" || kind === "typeAlias")
    },
    [diagram, model],
  )

  // The inspector is for symbols; selecting a group just highlights its relations.
  const selectedSymbol = useMemo(
    () => (selected ? model.symbols.find((s) => s.id === selected) : undefined),
    [model, selected],
  )

  return (
    <div className="flex h-full">
      {state.patternsOpen && <PatternsPanel model={model} />}
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
          // Trackpad: two-finger scroll pans, pinch zooms.
          panOnScroll
          panOnScrollMode={PanOnScrollMode.Free}
          zoomOnScroll={false}
          zoomOnPinch
          zoomOnDoubleClick={false}
          onNodeClick={(_, node) => {
            select(node.id)
            if (node.type === "symbol") changes.ack([node.id])
          }}
          onPaneClick={() => {
            select(null)
            diagram.clearSpotlight()
          }}
          proOptions={{ hideAttribution: true }}
          onlyRenderVisibleElements={nodes.length > CULL_ABOVE}
        >
          <UmlMarkers />
          <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
          <Panel position="top-left">
            <DiagramToolbar
              edgeKinds={state.filters.edgeKinds}
              hideTests={state.filters.hideTests}
              showFunctions={state.filters.showFunctions}
              allGroupEdges={state.filters.allGroupEdges}
              onAllGroupEdges={(allGroupEdges) => diagram.setFilters({ allGroupEdges })}
              onSearch={() => setSearchOpen(true)}
              onEdgeKinds={diagram.setEdgeKinds}
              onHideTests={(hideTests) => diagram.setFilters({ hideTests })}
              onShowFunctions={(showFunctions) => diagram.setFilters({ showFunctions })}
              onExpandAll={() => diagram.setExpanded(expandOneLevel(model, state.filters, state.expanded))}
              onCollapseAll={diagram.collapseAll}
              changedCount={Object.keys(changes.changed).length}
              onNextChange={() => {
                const next = Object.entries(changes.changed).sort((a, b) => b[1] - a[1])[0]?.[0]
                if (next) reveal(next)
              }}
              onClearChanges={changes.clear}
              patternsOpen={state.patternsOpen}
              onTogglePatterns={diagram.togglePatterns}
            />
          </Panel>
          {!selectedSymbol && <Controls showInteractive={false} position="bottom-left" />}
          {/* The minimap redraws on every pan; past a few hundred nodes it costs more than it helps. */}
          {nodes.length <= MINIMAP_MAX_NODES && !selectedSymbol && (
            <MiniMap
              pannable
              zoomable
              position="bottom-right"
              nodeStrokeWidth={0}
              nodeColor={(node) => packageColor((node.data as { pkg?: string }).pkg).value}
            />
          )}
        </ReactFlow>
        {selectedSymbol && (
          <SymbolInspector model={model} symbol={selectedSymbol} onNavigate={reveal} onClose={() => select(null)} />
        )}
      </div>
      <SymbolSearch model={model} open={searchOpen} onOpenChange={setSearchOpen} onSelect={reveal} listen={active} />
    </div>
  )
}

export function DiagramCanvas({ model }: { model: ArchModel }) {
  return (
    <ReactFlowProvider>
      <Canvas model={model} />
    </ReactFlowProvider>
  )
}
