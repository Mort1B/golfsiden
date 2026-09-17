import { expectedFourBall, type FourBallInput, type FourBallEntry } from '../../../api/fourBall'
import type { StablefordScoringCard, StablefordHole } from '../../../api/stableford'
import { useAuth } from '../../auth/authContext'
import { useScoreQueue } from '../offline/context'
import { useScoreDrafts } from '../recovery/context'
import { hasLease, queueKey, type StablefordTarget } from '../offline/model'

export function useStablefordSync(card: StablefordScoringCard, hole: StablefordHole<FourBallEntry>, tournamentId: string) {
  const { session } = useAuth()
  const queue = useScoreQueue()
  const target: StablefordTarget = { protocol: 'stableford_v1', accountId: session?.user_id ?? '', tournamentId,
    roundId: card.round_id, owner: { type: 'player', id: card.owner.id }, holeId: hole.hole_id, holeNumber: hole.hole_number }
  const key = queueKey(target)
  const { store, drafts } = useScoreDrafts()
  const pending = queue.items.find(item => item.key === key && item.protocol === 'stableford_v1')
  const refreshing = queue.refreshing.find(value => value.item.key === key)
  const local = drafts.find(item => item.key === key)
  const current = local?.kind === 'stableford' ? local : null
  const server = hole.score
  const desired = current?.value ?? (pending?.protocol === 'stableford_v1' ? pending.desired : null)
    ?? (refreshing?.item.protocol === 'stableford_v1' ? refreshing.item.desired : null) ?? server?.input ?? null
  const setInput = (value: FourBallInput) => store.set({ kind: 'stableford', target, value }, expectedFourBall(server), queue.runtime)
  return { desired, server: server?.input ?? null, setInput, navigationLocked: current !== null,
    storageReady: !queue.loading && queue.error === null && (!refreshing || pending !== undefined),
    phase: current?.error ? 'failed' : current?.saving ? 'saving' : pending ? pending.phase !== 'queued' ? pending.phase : hasLease(pending) ? 'verifying' : 'queued' : refreshing ? 'refreshing' : 'idle',
    error: current?.error ?? null, retry: () => store.retry(key, queue.runtime),
    discard: () => store.discard(key) }
}
export type StablefordSync = ReturnType<typeof useStablefordSync>
