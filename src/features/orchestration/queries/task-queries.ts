import { useQuery } from "@tanstack/react-query"

import type { RunEvent } from "@/shared/bindings/RunEvent"
import type { Task } from "@/shared/bindings/Task"
import type { TaskDetail } from "@/shared/bindings/TaskDetail"
import { query } from "@/shared/ipc/client"

export const taskKeys = {
  all: ["tasks"] as const,
  list: (projectId: string) => [...taskKeys.all, "list", projectId] as const,
  detail: (taskId: string) => [...taskKeys.all, "detail", taskId] as const,
  log: (runId: string) => [...taskKeys.all, "log", runId] as const,
  diff: (taskId: string) => [...taskKeys.all, "diff", taskId] as const,
}

/** A project's orchestrated tasks, oldest first. */
export function useTasks(projectId: string) {
  return useQuery({
    queryKey: taskKeys.list(projectId),
    queryFn: () => query<Task[]>("list_tasks", { input: { projectId } }),
  })
}

export function useTask(taskId: string | null) {
  return useQuery({
    queryKey: taskKeys.detail(taskId ?? ""),
    queryFn: () => query<TaskDetail>("get_task", { input: { taskId } }),
    enabled: !!taskId,
  })
}

/** Every event of a run; live runs keep growing through task events. */
export function useRunLog(runId: string | null) {
  return useQuery({
    queryKey: taskKeys.log(runId ?? ""),
    queryFn: () => query<RunEvent[]>("get_run_log", { input: { runId } }),
    enabled: !!runId,
  })
}

export function useTaskDiff(taskId: string | null, enabled = true) {
  return useQuery({
    queryKey: taskKeys.diff(taskId ?? ""),
    queryFn: () => query<string>("get_task_diff", { input: { taskId } }),
    enabled: !!taskId && enabled,
  })
}
