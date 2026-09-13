import { useRef, useState } from 'react'
import { expectedFourBall, type FourBallInput, type FourBallScoringCard, type FourBallHole, type FourBallEntry } from '../../../api/fourBall'
import { useAuth } from '../../auth/authContext'
import { useScoreQueue } from '../offline/context'
import { queueDatabase, STORAGE_ERROR } from '../offline/database'
import { hasLease, queueKey, type FourBallTarget } from '../offline/model'

interface LocalWrite { key: string; value: FourBallInput; saving: boolean; error: string | null; sequence: number }
export function useFourBallSync(card: FourBallScoringCard, hole: FourBallHole<FourBallEntry>, playerId: string, tournamentId: string) {
  const { session } = useAuth()
  const queue = useScoreQueue()
  const target: FourBallTarget = { protocol: 'four_ball_v1', sideId: card.owner.id, accountId: session?.user_id ?? '', tournamentId,
    roundId: card.round_id, owner: { type: 'player', id: playerId }, holeId: hole.hole_id, holeNumber: hole.hole_number }
  const key = queueKey(target)
  const [local, setLocal] = useState<LocalWrite | null>(null)
  const sequence = useRef(0)
  const chain = useRef(Promise.resolve())
  const pending = queue.items.find(item => item.key === key && item.protocol === 'four_ball_v1')
  const refreshing = queue.refreshing.find(value => value.item.key === key)
  const current = local?.key === key ? local : null
  const server = hole.players.find(player => player.player_id === playerId)?.score ?? null
  const desired = current?.value ?? (pending?.protocol === 'four_ball_v1' ? pending.desired : null)
    ?? (refreshing?.item.protocol === 'four_ball_v1' ? refreshing.item.desired : null) ?? server?.input ?? null
  const setInput = (value: FourBallInput) => {
    if (!session || !queue.runtime.isCurrent()) return
    const id = ++sequence.current
    setLocal({ key, value, saving: true, error: null, sequence: id })
    chain.current = chain.current.then(async () => {
      try {
        if (!queue.runtime.isCurrent()) return
        await queueDatabase.enqueueFourBall(target, value, expectedFourBall(server))
        if (!queue.runtime.isCurrent()) return
        await queue.runtime.changed()
        if (queue.runtime.isCurrent()) setLocal(previous => previous?.sequence === id ? null : previous)
      } catch (error) {
        if (queue.runtime.isCurrent()) setLocal(previous => previous?.sequence === id
          ? { ...previous, saving: false, error: error instanceof Error ? error.message : STORAGE_ERROR } : previous)
      }
    })
  }
  return { desired, server: server?.input ?? null, setInput, navigationLocked: current !== null,
    storageReady: !queue.loading && queue.error === null && (!refreshing || pending !== undefined),
    phase: current?.error ? 'failed' : current?.saving ? 'saving' : pending ? pending.phase !== 'queued' ? pending.phase : hasLease(pending) ? 'verifying' : 'queued' : refreshing ? 'refreshing' : 'idle',
    error: current?.error ?? null, retry: () => { if (current) setInput(current.value) },
    discard: () => { if (current && !current.saving) setLocal(null) } }
}
export type FourBallSync = ReturnType<typeof useFourBallSync>
