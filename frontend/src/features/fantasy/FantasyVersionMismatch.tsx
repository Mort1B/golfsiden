import { useFantasyDrafts } from './fantasyDrafts'

export function FantasyVersionMismatch() {
  const { action } = useFantasyDrafts()
  return <div role="alert"><p>Poengene bruker en annen regelversjon enn spillet. Oppdater Fantasy før du viser poengene.</p><button type="button" disabled={action.pending || action.phase === 'refreshing'} onClick={() => void action.refresh()}>Oppdater regelversjon og poeng</button></div>
}
