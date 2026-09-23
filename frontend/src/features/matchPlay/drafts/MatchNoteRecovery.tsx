import { useMatchNoteDrafts } from './context'
export function MatchNoteRecovery({ roundId, matchId }: { roundId: string; matchId: string }) {
  const { store, drafts } = useMatchNoteDrafts()
  const local = drafts.filter(item => item.target.roundId === roundId && item.target.matchId === matchId)
  if (!local.length) return null
  return <section className="match-panel"><h3>Ulagrede lokale notater</h3>
    <p>Notatene er beholdt på denne siden. De er ikke lagret på enheten. Vent på oppdatert tilgang, eller forkast dem.</p>
    {local.map(item => <p key={item.key}><span>Lokalt notat {item.slot}: {item.value || 'tomt'}</span> · hull {item.hole}</p>)}
    {local.some(item => item.saving) && <p role="status">Kontrollerer lokal lagring …</p>}
    {local.map(item => item.error && <p role="alert" key={item.key}>{item.error}</p>)}
    <button type="button" disabled={local.some(item => item.saving)} onClick={() => store.discard(roundId, matchId)}>Forkast ulagrede notater</button>
  </section>
}
