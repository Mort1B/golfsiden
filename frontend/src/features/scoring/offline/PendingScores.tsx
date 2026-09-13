import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Link, useLocation } from 'react-router-dom'
import { useAuth } from '../../auth/authContext'
import { scoringSearch } from '../selection'
import { useScoreQueue } from './context'
import { hasLease, type PendingScore } from './model'
import { queueDatabase } from './database'
import { ConflictReview } from './ConflictReview'
import './pendingScores.css'

export function PendingScoreLink() {
  const queue = useScoreQueue()
  const auth = useAuth()
  const location = useLocation()
  if (!auth.session || location.pathname === '/score' || (queue.items.length === 0 && !queue.error)) return null
  return <Link className="pending-score-link" to="/score#pending-scores">Lokale scoreendringer{queue.items.length > 0 ? ` (${queue.items.length})` : ''}</Link>
}
export function PendingScores() {
  const queue = useScoreQueue()
  return <details className="pending-scores" id="pending-scores">
    <summary>Lokale scoreendringer ({queue.items.length}){queue.items.some(item => item.phase !== 'queued') ? ' · krever valg' : queue.error ? ' · lagringsfeil' : ''}</summary>
    {queue.loading && <p role="status">Leser lagrede endringer på denne enheten …</p>}
    {queue.error && <div role="alert"><p>{queue.error}</p><button type="button" onClick={() => void queue.runtime.wake()}>Prøv lokal lagring igjen</button></div>}
    {!queue.loading && !queue.error && queue.items.length === 0 && <p>Ingen ventende endringer på denne enheten.</p>}
    {queue.refreshing.length > 0 && <p role="status">Kontrollerer serverscore for {queue.refreshing.length} hull. Netto og nye endringer på disse hullene venter på et oppdatert scorekort.</p>}
    {!queue.online && <p role="status">Du er frakoblet. Lokalt lagrede endringer sendes når forbindelsen er tilbake.</p>}
    {queue.items.length > 0 && <>
      <p>Endringene beholdes på denne enheten hvis du logger ut. Bare denne kontoen kan sende dem videre.</p>
      <ul>{queue.items.map(item => <PendingRow key={item.key} item={item} />)}</ul>
    </>}
  </details>
}
function PendingRow({ item }: { item: PendingScore }) {
  const { runtime, online } = useScoreQueue()
  const [review, setReview] = useState(false)
  const [discard, setDiscard] = useState<PendingScore | null>(null)
  const active = hasLease(item)
  const mutation = useMutation({ gcTime: 0, retry: false, networkMode: 'always',
    mutationFn: async (action: { type: 'retry' | 'discard'; generation: string }) => {
      if (!runtime.isCurrent()) return
      if (action.type === 'retry') await queueDatabase.retry(item.key, action.generation)
      else await queueDatabase.resolve(item.key, action.generation, null)
      if (runtime.isCurrent()) await runtime.changed()
    },
  })
  const busy = active || mutation.isPending
  const href = `/score?${scoringSearch({ tournamentId: item.tournamentId, roundId: item.roundId,
    owner: item.owner, holeNumber: item.holeNumber, view: 'hole' }).toString()}`
  return <li>
    <div className="pending-score-title"><Link to={href}>Hull {item.holeNumber} · {item.owner.type === 'team' ? 'lagkort' : 'spillerkort'}</Link><strong>Lokalt: {item.desired} slag</strong></div>
    <p role="status">{active ? 'Sender til serveren …' : item.phase === 'conflict' ? 'En annen score er lagret på serveren. Velg hvilken du vil beholde.'
      : item.phase === 'blocked' ? 'Kan ikke leveres med gjeldende tilgang eller rundestatus. Den lokale endringen beholdes.'
        : item.attempts > 0 ? 'Lagret på denne enheten. Levering mislyktes; prøver automatisk igjen.' : 'Lagret på denne enheten. Venter på levering.'}</p>
    {mutation.error && <p role="alert">{mutation.error.message}</p>}
    <div className="pending-actions">
      {item.phase === 'conflict' ? <button type="button" disabled={!online || busy} onClick={() => setReview(true)}>Sammenlign scorer</button>
        : <button type="button" disabled={!online || busy} onClick={() => mutation.mutate({ type: 'retry', generation: item.generation })}>Prøv levering igjen</button>}
      <button type="button" disabled={busy} onClick={() => setDiscard(discard ? null : item)}>Forkast lokal endring</button>
    </div>
    {discard && <div className="pending-review">
      {discard.generation !== item.generation && <p role="alert">Endringen ble oppdatert i en annen fane. Avbryt og se gjennom den på nytt.</p>}
      <p>Fjern bare den lokale kopien av {discard.desired} slag på hull {item.holeNumber}? Dette angrer ikke en score som allerede kan ha nådd serveren.</p>
      <button type="button" disabled={busy || discard.generation !== item.generation} onClick={() => mutation.mutate({ type: 'discard', generation: discard.generation })}>Ja, fjern lokal kopi</button>
      <button type="button" disabled={mutation.isPending} onClick={() => setDiscard(null)}>Avbryt</button>
    </div>}
    {review && item.phase === 'conflict' && <ConflictReview key={item.generation} item={item} onClose={() => setReview(false)} />}
  </li>
}
