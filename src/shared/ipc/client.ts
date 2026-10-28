import { invoke } from "@tauri-apps/api/core"

/**
 * The only module that talks to Tauri IPC. Features call `query` for
 * reads and `command` for writes, mirroring the backend's CQRS split.
 */
export function query<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(name, args)
}

export function command<T = void>(name: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(name, args)
}
