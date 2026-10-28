import { useEffect, useRef, useState } from "react"
import { FitAddon } from "@xterm/addon-fit"
import { WebLinksAddon } from "@xterm/addon-web-links"
import { Terminal } from "@xterm/xterm"
import { openUrl } from "@tauri-apps/plugin-opener"
import { RotateCcw, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

import { attachTerminal, resizeTerminal, writeTerminal } from "../commands/terminal-commands"

// xterm needs concrete colors; these match the shadcn zinc dark tokens.
const THEME = {
  background: "#09090b",
  foreground: "#fafafa",
  cursor: "#fafafa",
  cursorAccent: "#09090b",
  selectionBackground: "#3f3f46",
  black: "#18181b",
  brightBlack: "#52525b",
  red: "#f87171",
  brightRed: "#fca5a5",
  green: "#4ade80",
  brightGreen: "#86efac",
  yellow: "#facc15",
  brightYellow: "#fde047",
  blue: "#60a5fa",
  brightBlue: "#93c5fd",
  magenta: "#c084fc",
  brightMagenta: "#d8b4fe",
  cyan: "#22d3ee",
  brightCyan: "#67e8f9",
  white: "#e4e4e7",
  brightWhite: "#fafafa",
}

interface TerminalViewProps {
  terminalId: string
  visible: boolean
  onRestart: () => void
  onClose: () => void
}

export function TerminalView({ terminalId, visible, onRestart, onClose }: TerminalViewProps) {
  const host = useRef<HTMLDivElement>(null)
  const termRef = useRef<Terminal | null>(null)
  const fitRef = useRef<FitAddon | null>(null)
  const [exitCode, setExitCode] = useState<number | null | undefined>(undefined)

  useEffect(() => {
    const el = host.current
    if (!el) return
    const term = new Terminal({
      theme: THEME,
      fontFamily: 'ui-monospace, "SF Mono", "JetBrains Mono", Menlo, monospace',
      fontSize: 12.5,
      lineHeight: 1.2,
      cursorBlink: true,
      allowProposedApi: true,
      scrollback: 10_000,
      macOptionIsMeta: true,
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.loadAddon(new WebLinksAddon((_, uri) => void openUrl(uri)))
    term.open(el)
    termRef.current = term
    fitRef.current = fit

    let disposed = false
    const syncSize = () => {
      if (disposed || el.clientWidth === 0 || el.clientHeight === 0) return
      fit.fit()
      void resizeTerminal(terminalId, term.cols, term.rows)
    }
    syncSize()

    void attachTerminal(terminalId, {
      onData: (bytes) => !disposed && term.write(bytes),
      onExit: (code) => !disposed && setExitCode(code),
    }).then(syncSize)

    const input = term.onData((data) => void writeTerminal(terminalId, data))
    const observer = new ResizeObserver(() => requestAnimationFrame(syncSize))
    observer.observe(el)

    return () => {
      disposed = true
      observer.disconnect()
      input.dispose()
      term.dispose()
      termRef.current = null
    }
  }, [terminalId])

  useEffect(() => {
    if (!visible) return
    requestAnimationFrame(() => {
      fitRef.current?.fit()
      termRef.current?.focus()
    })
  }, [visible])

  return (
    <div className={cn("absolute inset-0 bg-[#09090b] p-2", !visible && "invisible")}>
      <div ref={host} className="h-full w-full" />
      {exitCode !== undefined && (
        <div className="absolute inset-x-3 bottom-3 flex items-center gap-2 rounded-lg border bg-card p-2 pl-3 text-sm shadow-sm">
          <span className="text-muted-foreground">
            Process exited{exitCode !== null && <> with code <span className="font-mono">{exitCode}</span></>}
          </span>
          <Button size="sm" variant="outline" className="ml-auto h-7" onClick={onRestart}>
            <RotateCcw /> Restart
          </Button>
          <Button size="sm" variant="ghost" className="h-7" onClick={onClose}>
            <X /> Close
          </Button>
        </div>
      )}
    </div>
  )
}
