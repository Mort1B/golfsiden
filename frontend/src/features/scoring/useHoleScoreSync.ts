import { useRef, useState } from 'react'
import type { ScoreOwner } from '../../api/scorecards'
import type { ExpectedScore } from '../../api/scorecards/conditional'
import type { Round } from '../../api/types'
import { useAuth } from '../auth/authContext'
import type { ScoreSyncSnapshot } from './scoreSyncState'
import { useScoreQueue } from './offline/context'
import { queueDatabase, STORAGE_ERROR } from './offline/database'
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
interface LocalWrite { key: string; value: number; saving: boolean; error: string | null; sequence: number }

export function useHoleScoreSync(input: HoleScoreSyncInput) {
  const { session } = useAuth()
  const queue = useScoreQueue()
  const target = { accountId: session?.user_id ?? '', tournamentId: input.tournamentId, roundId: input.round.id,
    owner: input.owner, holeId: input.holeId, holeNumber: input.holeNumber }
  const key = queueKey(target)
  const [local, setLocal] = useState<LocalWrite | null>(null)
  const sequence = useRef(0)
  const chain = useRef(Promise.resolve())
  const pending = queue.items.find(item => item.key === key && item.protocol !== 'four_ball_v1')
  const refreshing = queue.refreshing.find(value => value.item.key === key && value.item.protocol !== 'four_ball_v1')
  const current = local?.key === key ? local : null
  const snapshot: ScoreSyncSnapshot = {
    scope: { roundId: input.round.id, owner: input.owner, holeId: input.holeId },
    serverValue: input.serverValue,
    desiredValue: current?.value ?? (pending?.protocol !== 'four_ball_v1' ? pending?.desired : undefined) ?? (refreshing?.item.protocol !== 'four_ball_v1' ? refreshing?.item.desired : undefined) ?? input.serverValue,
    phase: current?.error ? 'failed' : current?.saving ? 'saving' : pending ? pending.phase !== 'queued' ? pending.phase : hasLease(pending) ? 'verifying' : 'queued' : refreshing ? 'refreshing' : 'idle',
    error: current?.error ? { message: current.error, retryable: true, configuration: false } : null,
  }
  const setScore = (value: number) => {
    if (!session || !queue.runtime.isCurrent()) return
    const id = ++sequence.current
    setLocal({ key, value, saving: true, error: null, sequence: id })
    chain.current = chain.current.then(async () => {
      try {
        if (!queue.runtime.isCurrent()) return
        await queueDatabase.enqueue(target, value, input.expected)
        if (!queue.runtime.isCurrent()) return
        await queue.runtime.changed()
        if (queue.runtime.isCurrent()) setLocal(previous => previous?.sequence === id ? null : previous)
      } catch (error) {
        if (queue.runtime.isCurrent()) setLocal(previous => previous?.sequence === id
          ? { ...previous, saving: false, error: error instanceof Error ? error.message : STORAGE_ERROR } : previous)
      }
    })
  }
  return {
    snapshot,
    navigationLocked: current !== null,
    storageReady: !queue.loading && queue.error === null && (!refreshing || pending !== undefined),
    setScore,
    retry: () => { if (current) setScore(current.value) },
    discard: () => { if (current && !current.saving) setLocal(null) },
  }
}
