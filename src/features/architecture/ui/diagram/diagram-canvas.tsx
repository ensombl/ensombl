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

import { flattenLayout, toElkGraph, type Layout } from "../../layout/elk-graph"
import { runLayout } from "../../layout/run-layout"
import { useDiagram } from "../../model/diagram-store"
import { ancestorsOf, buildView, defaultExpanded, expandOneLevel } from "../../model/view"
import { SymbolInspector } from "../inspector/symbol-inspector"
import { DiagramToolbar } from "./diagram-toolbar"
import { GroupNodeComponent, type GroupFlowNode } from "./group-node"
import { SymbolNodeComponent, type SymbolFlowNode } from "./symbol-node"
import { SymbolSearch } from "./symbol-search"
import { UmlEdgeComponent, UmlMarkers, type UmlFlowEdge } from "./uml-edge"

const nodeTypes = { symbol: SymbolNodeComponent, group: GroupNodeComponent }
const edgeTypes = { uml: UmlEdgeComponent }

function Canvas({ model }: { model: ArchModel }) {
  const diagram = useDiagram()
  const { state, init, clearFocus, select } = diagram
  const active = useIsActiveWorkspace()
  const flow = useReactFlow()
  const [layout, setLayout] = useState<{ layout: Layout; routed: Set<string> } | null>(null)
  const [searchOpen, setSearchOpen] = useState(false)
  const fitted = useRef(false)

  useEffect(() => {
    if (!state.initialized) init(defaultExpanded(model, state.filters))
  }, [state.initialized, state.filters, init, model])

  const view = useMemo(
    () => buildView(model, { expanded: state.expanded, filters: state.filters }),
    [model, state.expanded, state.filters],
  )

  useEffect(() => {
    let cancelled = false
    const { graph, routed } = toElkGraph(view)
    runLayout(graph)
      .then((result) => {
        if (!cancelled) setLayout({ layout: flattenLayout(result), routed })
      })
      .catch((error) => console.error("layout failed", error))
    return () => {
      cancelled = true
    }
  }, [view])

  const selected = state.selected
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
        parentId: n.parent && boxes.has(n.parent) ? n.parent : undefined,
        selected: n.id === selected,
      }
      if (n.type === "group") {
        const node: GroupFlowNode = {
          ...base,
          type: "group",
          data: { label: n.label, level: n.level, expanded: n.expanded, symbolCount: n.symbolCount },
          zIndex: n.expanded ? -1 : 0,
          selectable: !n.expanded,
        }
        return [node]
      }
      const node: SymbolFlowNode = { ...base, type: "symbol", data: { symbol: n.symbol } }
      return [node]
    })
  }, [view, layout, selected])

  const edges = useMemo<UmlFlowEdge[]>(() => {
    if (!layout) return []
    const { nodes: boxes, routes } = layout.layout
    return view.edges
      .filter((e) => boxes.has(e.source) && boxes.has(e.target))
      .map((e) => {
        const touches = e.source === selected || e.target === selected
        return {
          id: e.id,
          source: e.source,
          target: e.target,
          type: "uml",
          zIndex: touches ? 10 : 1,
          data: {
            kind: e.kind,
            count: e.count,
            highlighted: touches,
            dimmed: selected !== null && !touches,
            points: layout.routed.has(e.id) ? routes.get(e.id) : undefined,
          },
        }
      })
  }, [view, layout, selected])

  // First layout: fit everything. Later: center on a revealed symbol.
  useEffect(() => {
    if (!layout) return
    if (!fitted.current) {
      fitted.current = true
      requestAnimationFrame(() => flow.fitView({ padding: 0.12, maxZoom: 1 }))
    }
    if (state.focus && layout.layout.nodes.has(state.focus)) {
      const id = state.focus
      clearFocus()
      requestAnimationFrame(() => flow.fitView({ nodes: [{ id }], duration: 500, maxZoom: 1.2, padding: 0.6 }))
    }
  }, [layout, flow, state.focus, clearFocus])

  const reveal = useCallback(
    (symbolId: string) => {
      const kind = model.symbols.find((s) => s.id === symbolId)?.kind
      diagram.reveal(symbolId, ancestorsOf(model, symbolId), kind === "function" || kind === "typeAlias")
    },
    [diagram, model],
  )

  const selectedSymbol = useMemo(
    () => (selected ? model.symbols.find((s) => s.id === selected) : undefined),
    [model, selected],
  )

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
          // Trackpad: two-finger scroll pans, pinch zooms.
          panOnScroll
          panOnScrollMode={PanOnScrollMode.Free}
          zoomOnScroll={false}
          zoomOnPinch
          zoomOnDoubleClick={false}
          onNodeClick={(_, node) => node.type === "symbol" && select(node.id)}
          onPaneClick={() => select(null)}
          proOptions={{ hideAttribution: true }}
          onlyRenderVisibleElements
        >
          <UmlMarkers />
          <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
          <Panel position="top-left">
            <DiagramToolbar
              edgeKinds={state.filters.edgeKinds}
              hideTests={state.filters.hideTests}
              showFunctions={state.filters.showFunctions}
              onSearch={() => setSearchOpen(true)}
              onEdgeKinds={diagram.setEdgeKinds}
              onHideTests={(hideTests) => diagram.setFilters({ hideTests })}
              onShowFunctions={(showFunctions) => diagram.setFilters({ showFunctions })}
              onExpandAll={() => diagram.setExpanded(expandOneLevel(model, state.filters, state.expanded))}
              onCollapseAll={diagram.collapseAll}
            />
          </Panel>
          <Controls showInteractive={false} position="bottom-left" />
          <MiniMap pannable zoomable position="bottom-right" nodeStrokeWidth={2} />
        </ReactFlow>
      </div>
      {selectedSymbol && (
        <SymbolInspector model={model} symbol={selectedSymbol} onNavigate={reveal} onClose={() => select(null)} />
      )}
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
