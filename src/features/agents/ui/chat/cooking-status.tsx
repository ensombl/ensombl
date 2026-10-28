import { useEffect, useState } from "react"

import { turnTokens, type Turn } from "../../model/chat-state"

// The pulsing star, played forwards then back.
const GLYPHS = ["·", "✢", "✳", "✶", "✻", "✽", "✻", "✶", "✳", "✢"]

const VERBS = [
  "Cooking",
  "Clanking",
  "Vibing",
  "Whirring",
  "Scheming",
  "Tinkering",
  "Simmering",
  "Marinating",
  "Noodling",
  "Percolating",
  "Ruminating",
  "Rewiring",
  "Refactoring reality",
  "Herding types",
  "Untangling",
  "Spelunking",
  "Wrangling",
  "Conjuring",
  "Bamboozling the compiler",
  "Consulting the UML",
  "Clanker-ing",
  "Beep-booping",
  "Overthinking",
  "Brewing",
  "Finagling",
  "Reticulating splines",
]

const elapsed = (ms: number) => {
  const s = Math.max(0, Math.floor(ms / 1000))
  if (s < 60) return `${s}s`
  const m = Math.floor(s / 60)
  return m < 60 ? `${m}m ${s % 60}s` : `${Math.floor(m / 60)}h ${m % 60}m`
}

const tokens = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n))

/** Claude Code-style status line: `✻ Clanking… (1m 12s · ↓ 3.4k tokens · esc to stop)`. */
export function CookingStatus({ turn, waiting }: { turn: Turn; waiting: boolean }) {
  const [now, setNow] = useState(() => Date.now())
  const [frame, setFrame] = useState(0)
  // Pick a starting verb per turn so each turn reads differently.
  const [verb, setVerb] = useState(() => Math.floor(turn.startedAt / 1000) % VERBS.length)

  useEffect(() => {
    const clock = setInterval(() => setNow(Date.now()), 1000)
    const words = setInterval(() => setVerb((v) => (v + 1 + Math.floor(Math.random() * 3)) % VERBS.length), 4000)
    return () => {
      clearInterval(clock)
      clearInterval(words)
    }
  }, [])

  useEffect(() => {
    if (waiting) return
    const spin = setInterval(() => setFrame((f) => (f + 1) % GLYPHS.length), 120)
    return () => clearInterval(spin)
  }, [waiting])

  const count = turnTokens(turn)
  return (
    <p className="flex items-baseline gap-2 font-mono text-xs text-[#d97757]" role="status">
      <span className="inline-block w-3 text-center text-sm">{waiting ? "✻" : GLYPHS[frame]}</span>
      <span className={waiting ? "" : "crm-shimmer"}>{waiting ? "Waiting for you" : VERBS[verb]}…</span>
      <span className="text-muted-foreground">
        ({elapsed(now - turn.startedAt)}
        {count > 0 && <> · ↓ {tokens(count)} tokens</>}
        {!waiting && <> · esc to stop</>})
      </span>
    </p>
  )
}
