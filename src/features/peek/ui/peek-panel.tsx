import { Code2, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Pane } from "@/shared/ui/pane"

import { usePeekStore, type PeekTarget } from "../model/peek-store"

export function PeekPanel({ target }: { target: PeekTarget }) {
  const close = usePeekStore((s) => s.close)

  return (
    <Pane
      title={target.title}
      icon={<Code2 />}
      actions={
        <>
          <span className="font-mono text-xs text-muted-foreground">
            {target.file}:{target.line}
          </span>
          <Button variant="ghost" size="icon" className="size-7" onClick={close}>
            <X />
          </Button>
        </>
      }
    >
      <div className="p-4 text-sm text-muted-foreground">Code peek arrives with #7.</div>
    </Pane>
  )
}
