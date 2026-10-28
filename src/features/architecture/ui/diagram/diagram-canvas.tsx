import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  Panel,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Node,
} from "@xyflow/react"

import type { ArchModel } from "@/shared/bindings/ArchModel"
import { useIsActiveWorkspace } from "@/shared/workspace/workspace-context"

import { flattenLayout, toElkGraph, type LaidOutNode } from "../../layout/elk-graph"
import { runLayout } from "../../layout/run-layout"
import { useDiagram } from "../../model/diagram-store"
import { ancestorsOf, buildView, defaultExpanded, expandOneLevel } from "../../model/view"
import { DiagramToolbar } from "./diagram-toolbar"
import { GroupNodeComponent, type GroupFlowNode } from "./group-node"
import { SymbolNodeComponent, type SymbolFlowNode } from "./symbol-node"
import { SymbolSearch } from "./symbol-search"
import { UmlEdgeComponent, UmlMarkers, type UmlFlowEdge } from "./uml-edge"

const nodeTypes = { symbol: SymbolNodeComponent, group: GroupNodeComponent }
const edgeTypes = { uml: UmlEdgeComponent }

function Canvas({ model }: { model: ArchModel }) {
  const diagram = useDiagram()
  const { state } = diagram
  const active = useIsActiveWorkspace()
  const flow = useReactFlow()
  const [layout, setLayout] = useState<Map<string, LaidOutNode> | null>(null)
  const [selected, setSelected] = useState<string | null>(null)
  const [searchOpen, setSearchOpen] = useState(false)
  const fitted = useRef(false)

  const { init } = diagram
  useEffect(() => {
    if (!state.initialized) init(defaultExpanded(model))
  }, [state.initialized, init, model])

  const view = useMemo(
    () => buildView(model, { expanded: state.expanded, open: state.open, filters: state.filters }),
    [model, state.expanded, state.open, state.filters],
  )

  useEffect(() => {
    let cancelled = false
    runLayout(toElkGraph(view))
      .then((result) => {
        if (!cancelled) setLayout(flattenLayout(result))
      })
      .catch((error) => console.error("layout failed", error))
    return () => {
      cancelled = true
    }
  }, [view])

  const nodes = useMemo<Node[]>(() => {
    if (!layout) return []
    return view.nodes.flatMap((n): Node[] => {
      const box = layout.get(n.id)
      if (!box) return []
      const base = {
        id: n.id,
        position: { x: box.x, y: box.y },
        width: box.width,
        height: box.height,
        parentId: n.parent && layout.has(n.parent) ? n.parent : undefined,
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
      const node: SymbolFlowNode = { ...base, type: "symbol", data: { symbol: n.symbol, open: n.open } }
      return [node]
    })
  }, [view, layout, selected])

  const edges = useMemo<UmlFlowEdge[]>(
    () =>
      layout
        ? view.edges
            .filter((e) => layout.has(e.source) && layout.has(e.target))
            .map((e) => {
              const highlighted = selected !== null && (e.source === selected || e.target === selected)
              return {
                id: e.id,
                source: e.source,
                target: e.target,
                type: "uml",
                zIndex: highlighted ? 10 : 1,
                data: { kind: e.kind, count: e.count, highlighted },
              }
            })
        : [],
    [view, layout, selected],
  )

  // First layout: fit everything. Later: center on a symbol revealed from search.
  const { clearFocus } = diagram
  useEffect(() => {
    if (!layout) return
    if (!fitted.current) {
      fitted.current = true
      requestAnimationFrame(() => flow.fitView({ padding: 0.15, maxZoom: 1 }))
    }
    if (state.focus && layout.has(state.focus)) {
      const id = state.focus
      clearFocus()
      requestAnimationFrame(() => {
        setSelected(id)
        flow.fitView({ nodes: [{ id }], duration: 500, maxZoom: 1.1, padding: 0.4 })
      })
    }
  }, [layout, flow, state.focus, clearFocus])

  const reveal = useCallback(
    (symbolId: string) => diagram.reveal(symbolId, ancestorsOf(model, symbolId)),
    [diagram, model],
  )

  return (
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
      onNodeClick={(_, node) => setSelected(node.id)}
      onPaneClick={() => setSelected(null)}
      proOptions={{ hideAttribution: true }}
      onlyRenderVisibleElements
    >
      <UmlMarkers />
      <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
      <Panel position="top-left">
        <DiagramToolbar
          edgeKinds={state.filters.edgeKinds}
          hideTests={state.filters.hideTests}
          onSearch={() => setSearchOpen(true)}
          onEdgeKinds={diagram.setEdgeKinds}
          onHideTests={diagram.setHideTests}
          onExpandAll={() => diagram.expandAll([...expandOneLevel(model, state.expanded)])}
          onCollapseAll={diagram.collapseAll}
        />
      </Panel>
      <Controls showInteractive={false} position="bottom-left" />
      <MiniMap pannable zoomable position="bottom-right" nodeStrokeWidth={2} />
      <SymbolSearch model={model} open={searchOpen} onOpenChange={setSearchOpen} onSelect={reveal} listen={active} />
    </ReactFlow>
  )
}

export function DiagramCanvas({ model }: { model: ArchModel }) {
  return (
    <ReactFlowProvider>
      <Canvas model={model} />
    </ReactFlowProvider>
  )
}
