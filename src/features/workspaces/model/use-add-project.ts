import { pickFolder } from "@/shared/platform/dialog"

import { useAddProject } from "../commands/workspace-commands"

/** Asks for a folder inside a git repo and adds it as a project. */
export function useAddProjectFlow() {
  const addProject = useAddProject()
  return {
    pending: addProject.isPending,
    run: async () => {
      const path = await pickFolder("Add a git repository")
      if (path) addProject.mutate({ path })
    },
  }
}
