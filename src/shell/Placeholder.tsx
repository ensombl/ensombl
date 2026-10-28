export function Placeholder({ title, hint }: { title: string; hint: string }) {
  return (
    <section className="pane">
      <header className="pane-header">{title}</header>
      <div className="pane-empty">{hint}</div>
    </section>
  );
}
