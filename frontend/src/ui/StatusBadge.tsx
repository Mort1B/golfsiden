const labels: Record<string, string> = {
  draft: 'Kladd', active: 'Aktiv', completed: 'Fullført', archived: 'Arkivert', open: 'Åpen', locked: 'Låst',
}

export function StatusBadge({ status }: { status: string }) {
  return <span className={`status status-${status}`}>{labels[status] ?? status}</span>
}
