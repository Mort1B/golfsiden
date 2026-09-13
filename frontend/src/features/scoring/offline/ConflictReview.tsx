import { useMutation, useQuery } from '@tanstack/react-query'
import { api } from '../../../api/client'
import { scoringKeys } from '../../../api/scorecards'
import { expectedScore } from '../../../api/scorecards/conditional'
import { scoreRequest } from '../../../api/scorecards/timeout'
import { useScoreQueue } from './context'
import { queueDatabase } from './database'
import { hasLease, type PendingScore } from './model'

export function ConflictReview({ item, onClose }: { item: PendingScore; onClose: () => void }) {
  const { runtime, online } = useScoreQueue()
  const query = useQuery({ queryKey: scoringKeys.scoring(item.accountId, item.roundId, item.owner),
    queryFn: ({ signal }) => scoreRequest(child => api.scorecardScoring(item.roundId, item.owner, child), signal),
    staleTime: 0, refetchOnMount: 'always', retry: false,
  })
  const hole = query.data?.holes.find(value => value.hole_id === item.holeId)
  const ready = online && query.isFetchedAfterMount && !query.isFetching && !query.error && hole !== undefined && !hasLease(item)
  const mutation = useMutation({ gcTime: 0, retry: false, networkMode: 'always',
    mutationFn: async (choice: 'local' | 'server') => {
      if (!ready || !hole || !runtime.isCurrent()) throw new Error('Oppdater serverscoren før du velger.')
      await queueDatabase.resolve(item.key, item.generation, choice === 'local' ? expectedScore(hole.score) : null)
      if (runtime.isCurrent()) { await runtime.changed(); onClose() }
    },
  })
  return <div className="pending-review">
    <h3>Velg score for hull {item.holeNumber}</h3>
    <p>Din lokale score: <strong>{item.desired} slag</strong></p>
    {!query.isFetchedAfterMount || query.isFetching ? <p role="status">Henter gjeldende serverscore …</p>
      : query.error ? <div role="alert"><p>Kunne ikke hente gjeldende serverscore. Den lokale endringen beholdes.</p><button type="button" onClick={() => void query.refetch()}>Hent serverscore igjen</button></div>
        : hole && <p>Score på serveren: <strong>{hole.score ? `${hole.score.gross_strokes} slag` : 'Ikke registrert'}</strong></p>}
    <p>En ny endring fra andre etter dette valget krever at du sammenligner igjen.</p>
    {mutation.error && <p role="alert">{mutation.error.message}</p>}
    <div className="pending-actions">
      <button type="button" disabled={!ready || mutation.isPending} onClick={() => mutation.mutate('server')}>Behold serverscoren</button>
      <button type="button" disabled={!ready || mutation.isPending} onClick={() => mutation.mutate('local')}>Bruk min lokale score</button>
      <button type="button" disabled={mutation.isPending} onClick={onClose}>Lukk sammenligning</button>
    </div>
  </div>
}
