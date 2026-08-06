export function PlaceholderPage({ title, state }: { title: string; state: string }) {
  return (
    <section className="page">
      <header className="page-header"><p className="brand">Guttas Golf</p><h1>{title}</h1></header>
      <div className="state-message">{state}</div>
    </section>
  )
}
