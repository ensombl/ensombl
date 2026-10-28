import {
  AppWindow,
  Boxes,
  Cable,
  Clock,
  Cloud,
  Component,
  Database,
  Filter,
  Globe,
  Layers,
  Plug,
  Radio,
  Route,
  Server,
  Shield,
  Webhook,
  Wrench,
  type LucideIcon,
} from "lucide-react"

import type { InteractionKind } from "@/shared/bindings/InteractionKind"
import type { Layer } from "@/shared/bindings/Layer"
import type { Role } from "@/shared/bindings/Role"
import type { SystemNode } from "@/shared/bindings/SystemNode"

export const LAYER_COLOR: Record<Layer, string> = {
  interface: "oklch(0.74 0.13 230)",
  application: "oklch(0.74 0.15 160)",
  domain: "oklch(0.72 0.16 300)",
  data: "oklch(0.77 0.15 75)",
  external: "oklch(0.66 0.01 286)",
}

export const ROLE_LABEL: Record<Role, string> = {
  controller: "Controller",
  gateway: "Gateway",
  job: "Job",
  router: "Router",
  middleware: "Middleware",
  page: "Page",
  uiComponent: "UI component",
  hook: "Hook",
  service: "Service",
  repository: "Repository",
  client: "Client",
  other: "Unclassified",
  channel: "Channel",
  external: "External",
}

const ROLE_ICON: Record<Role, LucideIcon> = {
  controller: Route,
  gateway: Radio,
  job: Clock,
  router: Route,
  middleware: Shield,
  page: AppWindow,
  uiComponent: Component,
  hook: Webhook,
  service: Boxes,
  repository: Database,
  client: Plug,
  other: Wrench,
  channel: Cable,
  external: Cloud,
}

export function NodeIcon({ node, className }: { node: Pick<SystemNode, "role" | "name">; className?: string }) {
  let Icon = ROLE_ICON[node.role]
  if (node.role === "external") {
    if (/database|dynamo|redis|cache|elastic/i.test(node.name)) Icon = Database
    else if (node.name.includes(".") || /http/i.test(node.name)) Icon = Globe
    else if (/queue|sqs|kafka|broker/i.test(node.name)) Icon = Server
  }
  return <Icon className={className} />
}

export const FeatureIcon = Layers
export const FilterIcon = Filter

export const KIND_STYLE: Record<InteractionKind | "aggregate", { color: string; dash?: string; label: string; width: number }> = {
  call: { color: "oklch(0.78 0.01 286)", label: "Calls", width: 1.5 },
  inject: { color: "oklch(0.6 0.01 286)", dash: "5 4", label: "Injects", width: 1.25 },
  emits: { color: "oklch(0.78 0.15 75)", dash: "2 4", label: "Emits", width: 1.5 },
  handles: { color: "oklch(0.78 0.15 75)", dash: "2 4", label: "Delivers", width: 1.5 },
  http: { color: "oklch(0.74 0.13 230)", label: "HTTP", width: 1.75 },
  renders: { color: "oklch(0.72 0.16 300)", label: "Renders", width: 1.25 },
  aggregate: { color: "oklch(0.7 0.01 286)", label: "Depends on", width: 1.5 },
}

export const markerFor = (kind: InteractionKind | "aggregate") => `sys-${kind}`

/** Arrowheads per interaction kind. */
export function SystemMarkers() {
  return (
    <svg className="absolute size-0">
      <defs>
        {Object.entries(KIND_STYLE).map(([kind, style]) => (
          <marker
            key={kind}
            id={markerFor(kind as InteractionKind)}
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerWidth="7"
            markerHeight="7"
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" fill={style.color} />
          </marker>
        ))}
      </defs>
    </svg>
  )
}
