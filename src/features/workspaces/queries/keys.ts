export const workspaceKeys = {
  all: ["workspaces"] as const,
  session: () => [...workspaceKeys.all, "session"] as const,
  worktrees: (projectId: string) => [...workspaceKeys.all, "worktrees", projectId] as const,
  branches: (projectId: string) => [...workspaceKeys.all, "branches", projectId] as const,
}
