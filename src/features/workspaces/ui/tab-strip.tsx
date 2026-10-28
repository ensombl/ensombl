import { Fragment } from "react"

import type { SessionView } from "@/shared/bindings/SessionView"

import { groupColors } from "../model/group-colors"
import { CreateWorktreeDialog } from "./create-worktree-dialog"
import { NewTabMenu } from "./new-tab-menu"
import { RemoveWorktreeDialog } from "./remove-worktree-dialog"
import { TabGroupChip } from "./tab-group-chip"
import { WorktreeTab } from "./worktree-tab"

/** Worktree tabs grouped by project, like browser tab groups. Lives in the title bar. */
export function TabStrip({ session }: { session: SessionView }) {
  const projects = session.groups.map((g) => g.project)

  return (
    <div role="tablist" data-tauri-drag-region className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto overflow-y-hidden [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
      {session.groups.map(({ project, tabs }) => (
        <Fragment key={project.id}>
          <div data-tauri-drag-region className="flex shrink-0 items-center gap-1 pl-1 first:pl-0">
            <TabGroupChip project={project} tabCount={tabs.length} />
            {!project.collapsed &&
              tabs.map((tab) => (
                <WorktreeTab
                  key={tab.worktree.id}
                  tab={tab}
                  active={tab.worktree.id === session.active}
                  accent={groupColors[project.color].line}
                />
              ))}
          </div>
        </Fragment>
      ))}
      <NewTabMenu projects={projects} />
      <CreateWorktreeDialog projects={projects} />
      <RemoveWorktreeDialog />
    </div>
  )
}
