import type { ReactNode } from "react"

import { cn } from "@/lib/utils"

interface PaneProps {
  title: string
  icon?: ReactNode
  actions?: ReactNode
  className?: string
  children: ReactNode
}

export function Pane({ title, icon, actions, className, children }: PaneProps) {
  return (
    <section className={cn("flex h-full min-h-0 flex-col bg-background", className)}>
      <header className="flex h-10 shrink-0 items-center gap-2 border-b px-3">
        <span className="text-muted-foreground [&_svg]:size-4">{icon}</span>
        <h2 className="text-sm font-medium">{title}</h2>
        <div className="ml-auto flex items-center gap-1">{actions}</div>
      </header>
      <div className="min-h-0 flex-1">{children}</div>
    </section>
  )
}

export function PaneEmpty({ icon, title, description, children }: {
  icon: ReactNode
  title: string
  description: string
  children?: ReactNode
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
      <div className="flex size-10 items-center justify-center rounded-lg border bg-muted text-muted-foreground [&_svg]:size-5">
        {icon}
      </div>
      <div className="space-y-1">
        <p className="text-sm font-medium">{title}</p>
        <p className="max-w-xs text-sm text-muted-foreground">{description}</p>
      </div>
      {children}
    </div>
  )
}
