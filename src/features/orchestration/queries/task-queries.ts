import { useQuery } from "@tanstack/react-query"

import type { RunEvent } from "@/shared/bindings/RunEvent"
import type { Task } from "@/shared/bindings/Task"
import type { TaskCoordination } from "@/shared/bindings/TaskCoordination"
import type { TaskDetail } from "@/shared/bindings/TaskDetail"
import type { Usage } from "@/shared/bindings/Usage"
import type { WorkflowInfo } from "@/shared/bindings/WorkflowInfo"
import { query } from "@/shared/ipc/client"

export const taskKeys = {
  all: ["tasks"] as const,
  list: (projectId: string) => [...taskKeys.all, "list", projectId] as const,
  detail: (taskId: string) => [...taskKeys.all, "detail", taskId] as const,
  log: (runId: string) => [...taskKeys.all, "log", runId] as const,
  diff: (taskId: string) => [...taskKeys.all, "diff", taskId] as const,
  coordination: (projectId: string) => [...taskKeys.all, "coordination", projectId] as const,
  usage: (projectId: string) => [...taskKeys.all, "usage", projectId] as const,
  workflows: (projectId: string) => [...taskKeys.all, "workflows", projectId] as const,
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

/** Overlaps, conflicts and staleness of a project's active tasks; polled while any run. */
export function useCoordination(projectId: string, live: boolean) {
  return useQuery({
    queryKey: taskKeys.coordination(projectId),
    queryFn: () => query<TaskCoordination[]>("get_coordination", { input: { projectId } }),
    refetchInterval: live ? 10_000 : false,
  })
}

const startOfToday = () => {
  const d = new Date()
  d.setHours(0, 0, 0, 0)
  return d.getTime()
}

/** What this project's runs cost, today (local time) and overall. */
export function useUsage(projectId: string) {
  return useQuery({
    queryKey: taskKeys.usage(projectId),
    queryFn: () => query<Usage>("get_usage", { input: { projectId, since: startOfToday() } }),
  })
}

/** Workflows defined in the project's `.ensombl/workflows`. */
export function useWorkflows(projectId: string, enabled: boolean) {
  return useQuery({
    queryKey: taskKeys.workflows(projectId),
    queryFn: () => query<WorkflowInfo[]>("list_workflows", { input: { projectId } }),
    enabled,
  })
}
