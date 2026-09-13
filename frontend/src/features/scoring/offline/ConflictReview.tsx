import { useMutation, useQuery } from '@tanstack/react-query'
import { stablefordApi, type StablefordScoringCard } from '../../../api/stableford'
import { fourBallApi, expectedFourBall, inputLabel, type FourBallScoringCard } from '../../../api/fourBall'
import type { ScoringScorecard } from '../../../api/scorecards'
import { api } from '../../../api/client'
import { scoringKeys } from '../../../api/scorecards'
import { expectedScore } from '../../../api/scorecards/conditional'
import { scoreRequest } from '../../../api/scorecards/timeout'
import { useScoreQueue } from './context'
import { queueDatabase } from './database'
import { hasLease, pendingOwner, pendingLabel, type PendingScore } from './model'

export function ConflictReview({ item, onClose }: { item: PendingScore; onClose: () => void }) {
  const { runtime, online } = useScoreQueue()
  const query = useQuery<ScoringScorecard | FourBallScoringCard | StablefordScoringCard>({ queryKey: scoringKeys.scoring(item.accountId, item.roundId, pendingOwner(item)),
    queryFn: ({ signal }) => scoreRequest<ScoringScorecard | FourBallScoringCard | StablefordScoringCard>(child => item.protocol === 'four_ball_v1' ? fourBallApi.scoring(item.roundId, item.sideId, child) : item.protocol === 'stableford_v1' ? stablefordApi.scoring(item.roundId, item.owner.id, child) : api.scorecardScoring(item.roundId, item.owner, child), signal),
    staleTime: 0, refetchOnMount: 'always', retry: false,
  })
  const card = query.data
  const fourHole = card && 'format' in card && card.format === 'four_ball_stroke_play' ? card.holes.find(value => value.hole_id === item.holeId)?.players.find(player => player.player_id === item.owner.id) : undefined
  const legacyHole = card && !('format' in card) ? card.holes.find(value => value.hole_id === item.holeId) : undefined
  const stablefordHole = card && 'format' in card && card.format === 'individual_stableford' ? card.holes.find(value => value.hole_id === item.holeId) : undefined
  const inputHole = stablefordHole ?? fourHole
  const hole = inputHole ?? legacyHole
  const expected = inputHole ? expectedFourBall(inputHole.score) : legacyHole ? expectedScore(legacyHole.score) : null
  const serverLabel = inputHole ? inputLabel(inputHole.score?.input ?? null) : legacyHole?.score ? `${legacyHole.score.gross_strokes} slag` : 'Ikke registrert'
  const ready = online && query.isFetchedAfterMount && !query.isFetching && !query.error && hole !== undefined && !hasLease(item)
  const mutation = useMutation({ gcTime: 0, retry: false, networkMode: 'always',
    mutationFn: async (choice: 'local' | 'server') => {
      if (!ready || !hole || !runtime.isCurrent()) throw new Error('Oppdater serverscoren før du velger.')
      await queueDatabase.resolve(item.key, item.generation, choice === 'local' ? expected : null)
      if (runtime.isCurrent()) { await runtime.changed(); onClose() }
    },
  })
  return <div className="pending-review">
    <h3>Velg score for hull {item.holeNumber}</h3>
    <p>Din lokale score: <strong>{pendingLabel(item)}</strong></p>
    {!query.isFetchedAfterMount || query.isFetching ? <p role="status">Henter gjeldende serverscore …</p>
      : query.error ? <div role="alert"><p>Kunne ikke hente gjeldende serverscore. Den lokale endringen beholdes.</p><button type="button" onClick={() => void query.refetch()}>Hent serverscore igjen</button></div>
        : hole && <p>Score på serveren: <strong>{serverLabel}</strong></p>}
    <p>En ny endring fra andre etter dette valget krever at du sammenligner igjen.</p>
    {mutation.error && <p role="alert">{mutation.error.message}</p>}
    <div className="pending-actions">
      <button type="button" disabled={!ready || mutation.isPending} onClick={() => mutation.mutate('server')}>Behold serverscoren</button>
      <button type="button" disabled={!ready || mutation.isPending} onClick={() => mutation.mutate('local')}>Bruk min lokale score</button>
      <button type="button" disabled={mutation.isPending} onClick={onClose}>Lukk sammenligning</button>
    </div>
  </div>
}
