import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { useEffect, useRef } from "react"

/** Subscribes to a backend event for the component's lifetime. */
export function useBackendEvent<T>(name: string, handler: (payload: T) => void) {
  const ref = useRef(handler)
  useEffect(() => {
    ref.current = handler
  })

  useEffect(() => {
    let unlisten: UnlistenFn | undefined
    let disposed = false
    listen<T>(name, (event) => ref.current(event.payload)).then((fn) => {
      if (disposed) fn()
      else unlisten = fn
    })
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [name])
}
