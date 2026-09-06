import { useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import type { Round, Tournament } from '../../../api/types'
import { privateWorkspaceKeys } from '../../../api/privateWorkspace'
import { roundLifecycleKeys } from '../../../api/roundLifecycle'
import { useAuth } from '../../auth/authContext'
import { OrganizerRound } from './OrganizerRound'
import './organizer.css'

interface Props {
  tournament: Tournament
  rounds: Round[] | undefined
  pending: boolean
  error: Error | null
  authorityRefreshing: boolean
  onRefresh: () => void
}

export function OrganizerSummary({ tournament, rounds, pending, error, authorityRefreshing, onRefresh }: Props) {
  const client = useQueryClient()
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const blocked = pending || !!error || authorityRefreshing || auth.loading || !!auth.error
  const refresh = () => {
    onRefresh()
    for (const round of rounds ?? []) {
      const key = round.status === 'draft' ? roundLifecycleKeys.validation(userId, round.id) : privateWorkspaceKeys.completion(userId, round.id)
      if (round.status !== 'locked') void client.invalidateQueries({ queryKey: key, exact: true })
    }
  }
  const actionable = (rounds ?? []).filter(round => round.status !== 'locked')
    .sort((a, b) => a.round_number - b.round_number || a.id.localeCompare(b.id))
  return <section className="organizer-summary" aria-labelledby="organizer-summary-heading">
    <header><h2 id="organizer-summary-heading">Dette trenger oppfølging</h2>
      <button type="button" onClick={refresh} disabled={pending || authorityRefreshing}>Oppdater oversikten</button>
    </header>
    {error ? <p role="alert">Rundene kunne ikke hentes. Oppdater oversikten for å prøve igjen.</p>
      : blocked ? <p role="status">Kontrollerer tilgang og rundestatus …</p>
        : !rounds?.length ? <p>Ingen runder å følge opp ennå.</p>
          : !actionable.length ? <p>Alle rundene er låst. Ingen runder trenger oppfølging. <Link to="#lifecycle">Se turneringsstatus</Link></p> : null}
    <ul className="organizer-rounds">{actionable.map(round => <OrganizerRound key={round.id} tournament={tournament} round={round} blocked={blocked} onRefresh={refresh} />)}</ul>
  </section>
}
