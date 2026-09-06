import { useQuery } from '@tanstack/react-query'
import type { RoundPairings } from '../../api/pairings'
import { api } from '../../api/client'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import { EmptyState, ErrorState, LoadingState } from '../../ui/AsyncState'
import { useAuth } from '../auth/authContext'
import { flightProgress } from './flightProgress'
import type { RoundCompletionValidation } from '../../api/scorecards'

export function FlightProgressPanel({ pairings, onRefreshRound }: {
  pairings: RoundPairings; onRefreshRound: () => void
}) {
  const { session } = useAuth()
  const progress = useQuery({
    queryKey: privateWorkspaceKeys.completion(session?.user_id ?? '', pairings.round_id),
    queryFn: () => api.completionValidation(pairings.round_id, pairings.scoring_format),
    enabled: Boolean(session) && pairings.status !== 'draft',
  })
  const retry = () => { onRefreshRound(); void progress.refetch() }
  return <section className="flight-progress" aria-labelledby="flight-progress-heading">
    <div className="section-heading"><h2 id="flight-progress-heading">Fremdrift per flight</h2></div>
    {pairings.status === 'draft' ? <p>Fremdrift blir tilgjengelig når runden åpnes.</p> :
      progress.error ? <ErrorState error={progress.error} onRetry={retry} /> :
        !progress.data ? <><LoadingState /><button className="retry-button" type="button" onClick={retry}>Oppdater fremdrift</button></> :
          <>
            {progress.isFetching && <LoadingState />}
            <ProgressContent pairings={pairings} progress={progress.data} onRetry={retry} />
          </>}
  </section>
}

function ProgressContent({ pairings, progress, onRetry }: {
  pairings: RoundPairings; progress: RoundCompletionValidation; onRetry: () => void
}) {
  let flights
  try { flights = flightProgress(pairings, progress) } catch (error) {
    return <ErrorState error={error instanceof Error ? error : new Error('Fremdrift er utilgjengelig.')} onRetry={onRetry} />
  }
  if (flights.length === 0 || progress.owners.length === 0) return <EmptyState>Ingen scorekort med flighttilknytning er tilgjengelige.</EmptyState>
  const visibleOnly = progress.visibility.mode === 'front_nine'
  return <>
    <p>{visibleOnly
      ? 'Kun hull 1–9 vises. Fullføring og bekreftelse er skjult.'
      : 'Hvert scorekort telles én gang. Lag med felles scorekort telles som ett kort.'}</p>
    <div className="team-grid">{flights.map((flight) => <article className="team-card round-group" key={flight.id}>
      <header><h3>{flight.name}</h3></header>
      {flight.owners.length === 0 ? <EmptyState>Ingen scorekort er knyttet til denne flighten.</EmptyState> : <>
        <p className="flight-progress-summary">{flight.scored}/{flight.required} {visibleOnly ? 'synlige hullregistreringer' : 'hullregistreringer'} fordelt på {flight.owners.length} scorekort</p>
        {!visibleOnly && <p className="flight-progress-summary">{flight.completed}/{flight.owners.length} scorekort med alle hull ført · {flight.confirmed}/{flight.owners.length} bekreftet</p>}
        <ol>{flight.owners.map((owner) => <li key={`${owner.owner.type}:${owner.owner.id}`}>
          <span>{owner.owner_name}</span>
          <p>{owner.holes_scored}/{owner.required_holes} {visibleOnly ? 'synlige hull' : 'hull'}
            {!visibleOnly && (owner.confirmed ? ' · Bekreftet' : owner.complete ? ' · Alle hull ført, ikke bekreftet' : '')}</p>
        </li>)}</ol>
      </>}
    </article>)}</div>
  </>
}
