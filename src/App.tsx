import { useEffect, useState } from "react";
import { ipc } from "./ipc";
import { Shell } from "./shell/Shell";
import { Placeholder } from "./shell/Placeholder";

export default function App() {
  const [version, setVersion] = useState<string>();

  useEffect(() => {
    ipc
      .appInfo()
      .then((info) => setVersion(info.version))
      .catch(() => setVersion(undefined));
  }, []);

  return (
    <div className="app">
      <header className="titlebar">
        <strong>CRM</strong>
        <span className="muted">Clanker Resource Management</span>
        {version && <span className="muted version">v{version}</span>}
      </header>
      <Shell
        diagram={<Placeholder title="Architecture" hint="Open a repository to see its UML." />}
        peek={<Placeholder title="Peek" hint="Click a member to peek at its code." />}
        agents={<Placeholder title="Agents" hint="Start claude, codex or a custom CLI." />}
      />
    </div>
  );
}
