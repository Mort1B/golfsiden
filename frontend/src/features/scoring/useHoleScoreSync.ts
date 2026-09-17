import type { ScoreOwner } from '../../api/scorecards'
import type { ExpectedScore } from '../../api/scorecards/conditional'
import type { Round } from '../../api/types'
import { useAuth } from '../auth/authContext'
import type { ScoreSyncSnapshot } from './scoreSyncState'
import { useScoreQueue } from './offline/context'
import { useScoreDrafts } from './recovery/context'
import { hasLease, queueKey } from './offline/model'

interface HoleScoreSyncInput {
  round: Round
  tournamentId: string
  owner: ScoreOwner
  holeId: string
  holeNumber: number
  serverValue: number | null
  expected: ExpectedScore
}

export function useHoleScoreSync(input: HoleScoreSyncInput) {
  const { session } = useAuth()
  const queue = useScoreQueue()
  const target = { accountId: session?.user_id ?? '', tournamentId: input.tournamentId, roundId: input.round.id,
    owner: input.owner, holeId: input.holeId, holeNumber: input.holeNumber }
  const key = queueKey(target)
  const { store, drafts } = useScoreDrafts()
  const pending = queue.items.find(item => item.key === key && item.protocol === undefined)
  const refreshing = queue.refreshing.find(value => value.item.key === key && value.item.protocol === undefined)
  const local = drafts.find(item => item.key === key)
  const current = local?.kind === 'legacy' ? local : null
  const snapshot: ScoreSyncSnapshot = {
    scope: { roundId: input.round.id, owner: input.owner, holeId: input.holeId },
    serverValue: input.serverValue,
    desiredValue: current?.value ?? (pending?.protocol === undefined ? pending?.desired : undefined) ?? (refreshing?.item.protocol === undefined ? refreshing?.item.desired : undefined) ?? input.serverValue,
    phase: current?.error ? 'failed' : current?.saving ? 'saving' : pending ? pending.phase !== 'queued' ? pending.phase : hasLease(pending) ? 'verifying' : 'queued' : refreshing ? 'refreshing' : 'idle',
    error: current?.error ? { message: current.error, retryable: true, configuration: false } : null,
  }
  const setScore = (value: number) => store.set({ kind: 'legacy', target, value }, input.expected, queue.runtime)
  return {
    snapshot,
    navigationLocked: current !== null,
    storageReady: !queue.loading && queue.error === null && (!refreshing || pending !== undefined),
    setScore,
    retry: () => store.retry(key, queue.runtime),
    discard: () => store.discard(key),
  }
}
