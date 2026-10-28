import { useMemo, useState } from "react"
import { Bot, Loader2, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import type { AgentKind } from "@/shared/bindings/AgentKind"
import type { ChatInfo } from "@/shared/bindings/ChatInfo"
import type { TerminalInfo } from "@/shared/bindings/TerminalInfo"
import { Pane, PaneEmpty } from "@/shared/ui/pane"
import { useIsActiveWorkspace, useWorkspace } from "@/shared/workspace/workspace-context"

import { useModel } from "@/features/architecture/queries/model-queries"
import { usePatterns } from "@/features/patterns/queries/pattern-queries"

import { useCloseChat, useStartChat } from "../commands/chat-commands"
import { useCloseTerminal, useStartTerminal, writeTerminal } from "../commands/terminal-commands"
import { bracketedPaste } from "../model/context"
import { useChatStatus } from "../model/chat-status"
import { useAgents, useChats, useTerminals } from "../queries/agent-queries"
import { AgentIcon } from "./agent-icon"
import { ChatView } from "./chat/chat-view"
import { Composer } from "./composer"
import { NewAgentMenu } from "./new-agent-menu"
import { TerminalView } from "./terminal-view"

type AgentTab =
  | { type: "chat"; id: string; title: string; info: ChatInfo }
  | { type: "terminal"; id: string; title: string; info: TerminalInfo }

/** Agents running in this worktree, one tab each: native chats and terminals. */
export function AgentsPane() {
  const worktree = useWorkspace()
  const workspaceActive = useIsActiveWorkspace()
  const { data: agents = [] } = useAgents()
  const { data: chats = [] } = useChats(worktree.id)
  const { data: terminals = [] } = useTerminals(worktree.id)
  const startChat = useStartChat()
  const startTerminal = useStartTerminal()
  const closeChat = useCloseChat(worktree.id)
  const closeTerminal = useCloseTerminal(worktree.id)
  const [selected, setSelected] = useState<string | null>(null)
  const { data: model = null } = useModel(worktree.id)
  const { data: patternViews } = usePatterns(worktree.id, !!model)
  const patterns = useMemo(() => (patternViews ?? []).filter((p) => p.decision !== "dismissed").map((p) => p.pattern), [patternViews])

  const tabs = useMemo<AgentTab[]>(
    () => [
      ...chats.map((c) => ({ type: "chat" as const, id: c.id, title: c.title, info: c })),
      ...terminals.map((t) => ({ type: "terminal" as const, id: t.id, title: t.title, info: t })),
    ],
    [chats, terminals],
  )
  const active = tabs.find((t) => t.id === selected) ?? tabs[tabs.length - 1]
  const kindOf = (tab: AgentTab): AgentKind =>
    tab.type === "chat" ? tab.info.agent : (agents.find((a) => a.profile.id === tab.info.profileId)?.profile.kind ?? "custom")
  const statuses = useChatStatus((s) => s.byChat)
  const statusOf = (tab: AgentTab) =>
    tab.type === "chat"
      ? (statuses[tab.id] ?? { busy: tab.info.busy, running: tab.info.running, needsInput: false })
      : { busy: false, running: tab.info.running, needsInput: false }
  const pending = startChat.isPending || startTerminal.isPending

  const launch = (profileId: string, resume?: string) => {
    const profile = agents.find((a) => a.profile.id === profileId)?.profile
    if (profile?.mode === "chat") {
      startChat.mutate({ worktreeId: worktree.id, profileId, resume }, { onSuccess: (info) => setSelected(info.id) })
    } else {
      startTerminal.mutate({ worktreeId: worktree.id, profileId, cols: 100, rows: 30 }, { onSuccess: (info) => setSelected(info.id) })
    }
  }

  const close = (tab: AgentTab) => {
    if (tab.type === "chat") closeChat.mutate(tab.id)
    else closeTerminal.mutate(tab.id)
    if (tab.id === active?.id) setSelected(null)
  }

  return (
    <Pane title="Agents" icon={<Bot />} actions={<NewAgentMenu agents={agents} onStart={launch} />}>
      {tabs.length === 0 ? (
        <PaneEmpty
          icon={<Bot />}
          title="No agent running"
          description="Agents run in this worktree with your existing CLI login. No API keys needed."
        >
          <div className="flex flex-wrap justify-center gap-2">
            {agents
              .filter((a) => a.available)
              .map(({ profile }) => (
                <Button key={profile.id} size="sm" variant="outline" onClick={() => launch(profile.id)} disabled={pending}>
                  <AgentIcon kind={profile.kind} /> {profile.label}
                </Button>
              ))}
          </div>
        </PaneEmpty>
      ) : (
        <div className="flex h-full flex-col">
          <div role="tablist" className="flex h-9 shrink-0 items-center gap-1 overflow-x-auto border-b px-2">
            {tabs.map((t) => {
              const status = statusOf(t)
              return (
                <div
                  key={t.id}
                  role="tab"
                  aria-selected={t.id === active?.id}
                  onClick={() => setSelected(t.id)}
                  className={cn(
                    "group flex h-7 shrink-0 cursor-default items-center gap-1.5 rounded-md pr-1 pl-2 text-xs",
                    t.id === active?.id ? "bg-accent text-accent-foreground" : "text-muted-foreground hover:bg-accent/50",
                  )}
                >
                  {status.busy && !status.needsInput ? (
                    <Loader2 className="size-3.5 animate-spin" />
                  ) : (
                    <AgentIcon kind={kindOf(t)} className="size-3.5" />
                  )}
                  <span className={cn(!status.running && "line-through opacity-60")}>{t.title}</span>
                  {status.needsInput && <span className="size-1.5 rounded-full bg-amber-400" title="Waiting for your approval" />}
                  <button
                    type="button"
                    aria-label={`Close ${t.title}`}
                    onClick={(e) => {
                      e.stopPropagation()
                      close(t)
                    }}
                    className="flex size-4 items-center justify-center rounded-sm opacity-0 group-hover:opacity-100 hover:bg-muted-foreground/20"
                  >
                    <X className="size-3" />
                  </button>
                </div>
              )
            })}
          </div>
          <div className="relative min-h-0 flex-1">
            {tabs.map((t) =>
              t.type === "chat" ? (
                <ChatView
                  key={t.id}
                  chat={t.info}
                  visible={workspaceActive && t.id === active?.id}
                  model={model}
                  patterns={patterns}
                  onRestart={(sessionId) => {
                    close(t)
                    launch(t.info.profileId, sessionId ?? undefined)
                  }}
                  onClose={() => close(t)}
                />
              ) : (
                <TerminalView
                  key={t.id}
                  terminalId={t.id}
                  visible={workspaceActive && t.id === active?.id}
                  onRestart={() => {
                    close(t)
                    launch(t.info.profileId)
                  }}
                  onClose={() => close(t)}
                />
              ),
            )}
          </div>
          {active?.type === "terminal" && (
            <Composer
              model={model}
              patterns={patterns}
              disabled={!active.info.running}
              placeholder={`Message ${active.title}… @ to tag a class, method or file`}
              onSend={async (prompt) => {
                await writeTerminal(active.id, bracketedPaste(prompt))
                // Give the TUI a beat to take the paste before submitting it.
                setTimeout(() => void writeTerminal(active.id, "\r"), 60)
              }}
            />
          )}
        </div>
      )}
    </Pane>
  )
}
