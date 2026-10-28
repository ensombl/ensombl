import { useQueryClient } from "@tanstack/react-query"

import type { RunEvent } from "@/shared/bindings/RunEvent"
import type { RunEventPayload } from "@/shared/bindings/RunEventPayload"
import { useBackendEvent } from "@/shared/ipc/events"

import { taskKeys } from "../queries/task-queries"

/** Run events that change what a task detail shows, beyond its log. */
const CHANGES_DETAIL = new Set<RunEvent["type"]>(["permission", "permissionResolved", "turnComplete", "exited"])

/** Keeps task queries live: task changes refetch, run events extend open logs. */
export function useTaskEvents() {
  const client = useQueryClient()
  useBackendEvent<{ id: string }>("tasks://changed", (task) => {
    client.invalidateQueries({ queryKey: [...taskKeys.all, "list"] })
    client.invalidateQueries({ queryKey: taskKeys.detail(task.id) })
  })
  useBackendEvent<RunEventPayload>("tasks://run", ({ taskId, runId, event }) => {
    client.setQueryData<RunEvent[]>(taskKeys.log(runId), (log) => (log ? [...log, event] : log))
    if (CHANGES_DETAIL.has(event.type)) client.invalidateQueries({ queryKey: taskKeys.detail(taskId) })
  })
}
