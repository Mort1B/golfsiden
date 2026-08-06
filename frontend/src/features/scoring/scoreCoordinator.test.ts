import { describe, expect, it, vi } from 'vitest'
import { ApiHttpError } from '../../api/http'
import { ScoreCoordinator } from './scoreCoordinator'

const scope = {
  roundId: 'round-a',
  owner: { type: 'player' as const, id: 'player-a' },
  holeId: 'hole-a',
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail })
  return { promise, resolve, reject }
}

async function settle(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}

describe('ScoreCoordinator', () => {
  it('runs one write at a time and coalesces rapid taps to the latest value', async () => {
    const firstSave = deferred<void>()
    const secondSave = deferred<void>()
    const saves: number[] = []
    const save = vi.fn((value: number) => {
      saves.push(value)
      return saves.length === 1 ? firstSave.promise : secondSave.promise
    })
    const verify = vi.fn(async () => saves.at(-1) ?? null)
    const coordinator = new ScoreCoordinator(scope, 4, { save, verify, terminalRefresh: verify })

    coordinator.setDesired(5)
    coordinator.setDesired(6)
    coordinator.setDesired(7)
    expect(save).toHaveBeenCalledTimes(1)
    expect(coordinator.current().desiredValue).toBe(7)
    expect(coordinator.current().phase).toBe('queued')

    firstSave.resolve()
    await settle()
    expect(save).toHaveBeenCalledTimes(2)
    expect(saves).toEqual([5, 7])
    secondSave.resolve()
    await settle()
    expect(coordinator.current()).toMatchObject({ serverValue: 7, desiredValue: 7, phase: 'synced' })
  })

  it('waits for final refetch agreement before reporting synchronized', async () => {
    const verification = deferred<number | null>()
    const coordinator = new ScoreCoordinator(scope, null, {
      save: async () => undefined,
      verify: () => verification.promise,
      terminalRefresh: () => verification.promise,
    })
    coordinator.setDesired(4)
    await settle()
    expect(coordinator.current().phase).toBe('verifying')
    verification.resolve(4)
    await settle()
    expect(coordinator.current().phase).toBe('synced')
  })

  it('preserves the latest intended value after a failure and retries it', async () => {
    let attempts = 0
    const save = vi.fn(async () => {
      attempts += 1
      if (attempts === 1) throw new Error('offline')
    })
    const coordinator = new ScoreCoordinator(scope, 4, { save, verify: async () => 6, terminalRefresh: async () => 4 })
    coordinator.setDesired(6)
    await settle()
    expect(coordinator.current()).toMatchObject({ desiredValue: 6, phase: 'failed' })
    coordinator.retry()
    await settle()
    expect(save).toHaveBeenCalledTimes(2)
    expect(coordinator.current()).toMatchObject({ serverValue: 6, phase: 'synced' })
  })

  it('discards failed intent back to the latest server value', async () => {
    const coordinator = new ScoreCoordinator(scope, 4, {
      save: async () => { throw new Error('offline') },
      verify: async () => 4,
      terminalRefresh: async () => 4,
    })
    coordinator.setDesired(5)
    await settle()
    coordinator.acceptServerValue(3)
    coordinator.discard()
    expect(coordinator.current()).toMatchObject({ serverValue: 3, desiredValue: 3, phase: 'idle' })
  })

  it('treats a lock conflict as terminal and discards local intent', async () => {
    const terminalRefresh = vi.fn(async () => 4)
    const coordinator = new ScoreCoordinator(scope, 4, {
      save: async () => { throw new ApiHttpError(409, 'round_not_editable', 'locked') },
      verify: async () => 5,
      terminalRefresh,
    })
    coordinator.setDesired(5)
    await settle()
    expect(terminalRefresh).toHaveBeenCalledOnce()
    expect(coordinator.current()).toMatchObject({ desiredValue: 4, phase: 'idle' })
  })

  it('marks a missing configured scorer as non-retryable', async () => {
    const coordinator = new ScoreCoordinator(scope, 4, {
      save: async () => { throw new ApiHttpError(404, 'not_found', 'missing') },
      verify: async () => 5,
      terminalRefresh: async () => 4,
    })
    coordinator.setDesired(5)
    await settle()
    expect(coordinator.current().error).toMatchObject({ retryable: false, configuration: true })
  })

  it('keeps hole and owner coordinators isolated', async () => {
    const secondScope = { ...scope, owner: { type: 'team' as const, id: 'team-b' }, holeId: 'hole-b' }
    const first = new ScoreCoordinator(scope, 4, { save: async () => undefined, verify: async () => 5, terminalRefresh: async () => 4 })
    const second = new ScoreCoordinator(secondScope, 3, { save: async () => undefined, verify: async () => 3, terminalRefresh: async () => 3 })
    first.setDesired(5)
    await settle()
    expect(first.current().desiredValue).toBe(5)
    expect(second.current()).toMatchObject({ desiredValue: 3, phase: 'idle' })
  })

  it('continues publishing after a subscription cleanup and replay', async () => {
    const coordinator = new ScoreCoordinator(scope, 4, {
      save: async () => undefined,
      verify: async () => 5,
      terminalRefresh: async () => 4,
    })
    const firstUnsubscribe = coordinator.subscribe(() => undefined)
    firstUnsubscribe()
    const phases: string[] = []
    coordinator.subscribe((snapshot) => phases.push(snapshot.phase))

    coordinator.setDesired(5)
    await settle()

    expect(phases).toContain('verifying')
    expect(phases.at(-1)).toBe('synced')
  })
})
