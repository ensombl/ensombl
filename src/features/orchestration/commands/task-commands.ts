import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import type { CreateTask } from "@/shared/bindings/CreateTask"
import type { PermissionDecision } from "@/shared/bindings/PermissionDecision"
import type { Task } from "@/shared/bindings/Task"
import { command } from "@/shared/ipc/client"

import { taskKeys } from "../queries/task-queries"

/** A task command: refreshes the task lists, failures show a toast. */
function useTaskMutation<I, O>(name: string, what: string) {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (input: I) => command<O>(name, { input }),
    onSettled: () => client.invalidateQueries({ queryKey: taskKeys.all }),
    onError: (error) => toast.error(`Couldn't ${what}`, { description: String(error) }),
  })
}

type ById = { taskId: string }

export function useTaskCommands() {
  return {
    create: useTaskMutation<CreateTask, Task>("create_task", "create the task"),
    queue: useTaskMutation<ById, Task>("queue_task", "queue the task"),
    cancel: useTaskMutation<ById, Task>("cancel_task", "cancel the task"),
    retry: useTaskMutation<ById, Task>("retry_task", "retry the task"),
    approve: useTaskMutation<ById, Task>("approve_task", "merge the task"),
    remove: useTaskMutation<ById, void>("delete_task", "delete the task"),
    update: useTaskMutation<ById, void>("update_task_branch", "update the branch"),
    respond: useTaskMutation<ById & { requestId: string; decision: PermissionDecision }, void>("respond_to_task", "answer the agent"),
    followUp: useTaskMutation<ById & { prompt: string }, Task>("send_follow_up", "send the follow-up"),
  }
}
