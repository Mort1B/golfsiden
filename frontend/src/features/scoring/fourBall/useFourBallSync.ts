import { expectedFourBall, type FourBallInput, type FourBallScoringCard, type FourBallHole, type FourBallEntry } from '../../../api/fourBall'
import { useAuth } from '../../auth/authContext'
import { useScoreQueue } from '../offline/context'
import { useScoreDrafts } from '../recovery/context'
import { hasLease, queueKey, type FourBallTarget } from '../offline/model'

export function useFourBallSync(card: FourBallScoringCard, hole: FourBallHole<FourBallEntry>, playerId: string, tournamentId: string) {
  const { session } = useAuth()
  const queue = useScoreQueue()
  const target: FourBallTarget = { protocol: 'four_ball_v1', sideId: card.owner.id, accountId: session?.user_id ?? '', tournamentId,
    roundId: card.round_id, owner: { type: 'player', id: playerId }, holeId: hole.hole_id, holeNumber: hole.hole_number }
  const key = queueKey(target)
  const { store, drafts } = useScoreDrafts()
  const pending = queue.items.find(item => item.key === key && item.protocol === 'four_ball_v1')
  const refreshing = queue.refreshing.find(value => value.item.key === key)
  const local = drafts.find(item => item.key === key)
  const current = local?.kind === 'four_ball' ? local : null
  const server = hole.players.find(player => player.player_id === playerId)?.score ?? null
  const desired = current?.value ?? (pending?.protocol === 'four_ball_v1' ? pending.desired : null)
    ?? (refreshing?.item.protocol === 'four_ball_v1' ? refreshing.item.desired : null) ?? server?.input ?? null
  const setInput = (value: FourBallInput) => store.set({ kind: 'four_ball', slot: card.partners[0].player_id === playerId ? 1 : 2, target, value }, expectedFourBall(server), queue.runtime)
  return { desired, server: server?.input ?? null, setInput, navigationLocked: current !== null,
    storageReady: !queue.loading && queue.error === null && (!refreshing || pending !== undefined),
    phase: current?.error ? 'failed' : current?.saving ? 'saving' : pending ? pending.phase !== 'queued' ? pending.phase : hasLease(pending) ? 'verifying' : 'queued' : refreshing ? 'refreshing' : 'idle',
    error: current?.error ?? null, retry: () => store.retry(key, queue.runtime),
    discard: () => store.discard(key) }
}
export type FourBallSync = ReturnType<typeof useFourBallSync>
