import { Group, Panel, Separator } from "react-resizable-panels";
import type { ReactNode } from "react";

interface ShellProps {
  diagram: ReactNode;
  agents: ReactNode;
  peek: ReactNode;
}

/** Three-pane layout: the diagram is always on screen, agents on the right, peek in a drawer. */
export function Shell({ diagram, agents, peek }: ShellProps) {
  return (
    <Group orientation="horizontal" className="shell">
      <Panel defaultSize="68" minSize="30">
        <Group orientation="vertical">
          <Panel defaultSize="75" minSize="20">
            {diagram}
          </Panel>
          <Separator className="sep sep-h" />
          <Panel defaultSize="25" minSize="0" collapsible>
            {peek}
          </Panel>
        </Group>
      </Panel>
      <Separator className="sep sep-v" />
      <Panel defaultSize="32" minSize="20">
        {agents}
      </Panel>
    </Group>
  );
}
