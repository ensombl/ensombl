import { Channel } from "@tauri-apps/api/core"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import type { StartTerminal } from "@/shared/bindings/StartTerminal"
import type { TerminalInfo } from "@/shared/bindings/TerminalInfo"
import { command } from "@/shared/ipc/client"

import { agentKeys } from "../queries/agent-queries"

/** Output stream message: raw PTY bytes, or the exit notice. */
export type TerminalMessage = ArrayBuffer | { exit: number | null }

export interface TerminalStream {
  onData: (bytes: Uint8Array) => void
  onExit: (code: number | null) => void
}

function channelFor(stream: TerminalStream) {
  const channel = new Channel<TerminalMessage>()
  channel.onmessage = (message) => {
    if (message instanceof ArrayBuffer) stream.onData(new Uint8Array(message))
    else if (Array.isArray(message)) stream.onData(new Uint8Array(message as number[]))
    else stream.onExit(message.exit)
  }
  return channel
}

const ignore: TerminalStream = { onData: () => {}, onExit: () => {} }

/** Re-attaches a view to a terminal; the backend replays scrollback first. */
export const attachTerminal = (terminalId: string, stream: TerminalStream) =>
  command<TerminalInfo>("attach_terminal", { input: { terminalId }, output: channelFor(stream) })

export const writeTerminal = (terminalId: string, data: string) =>
  command("write_terminal", { input: { terminalId, data } })

export const resizeTerminal = (terminalId: string, cols: number, rows: number) =>
  command("resize_terminal", { input: { terminalId, cols, rows } })

export function useStartTerminal() {
  const client = useQueryClient()
  return useMutation({
    // The view attaches itself and gets everything replayed, so the start stream is ignored.
    mutationFn: (input: StartTerminal) =>
      command<TerminalInfo>("start_terminal", { input, output: channelFor(ignore) }),
    onSuccess: (info) => client.invalidateQueries({ queryKey: agentKeys.terminals(info.worktreeId) }),
    onError: (error) => toast.error("Couldn't start agent", { description: String(error) }),
  })
}

export function useCloseTerminal(worktreeId: string) {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (terminalId: string) => command("close_terminal", { input: { terminalId } }),
    onSettled: () => client.invalidateQueries({ queryKey: agentKeys.terminals(worktreeId) }),
  })
}
