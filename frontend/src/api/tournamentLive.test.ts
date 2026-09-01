import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  subscribeTournamentLive,
  tournamentLiveEventTypes,
  type TournamentLiveSource,
} from './tournamentLive'

class FakeSource implements TournamentLiveSource {
  readonly listeners = new Map<string, EventListener>()
  readonly close = vi.fn()

  addEventListener(type: string, listener: EventListener): void {
    this.listeners.set(type, listener)
  }

  emit(type: string): void {
    this.listeners.get(type)?.(new Event(type))
  }
}

afterEach(async () => {
  await Promise.resolve()
})

describe('tournament live subscription', () => {
  it('opens one credentialed target stream, handles all events, and closes after cleanup', async () => {
    const source = new FakeSource()
    const factory = vi.fn(() => source)
    const invalidate = vi.fn()

    const unsubscribe = subscribeTournamentLive('user-one', 'tour-one', invalidate, factory)

    expect(factory).toHaveBeenCalledWith('/api/tournaments/tour-one/live', { withCredentials: true })
    expect([...source.listeners.keys()]).toEqual([...tournamentLiveEventTypes, 'open'])
    for (const eventType of tournamentLiveEventTypes) source.emit(eventType)
    expect(invalidate).toHaveBeenCalledTimes(5)
    expect(source.listeners.has('invitation')).toBe(true)

    unsubscribe()
    await Promise.resolve()
    expect(source.close).toHaveBeenCalledOnce()
  })

  it('invalidates when the stream initially opens and whenever it reconnects', () => {
    const source = new FakeSource()
    const invalidate = vi.fn()
    const unsubscribe = subscribeTournamentLive(
      'user-open',
      'tour-open',
      invalidate,
      () => source,
    )

    source.emit('open')
    source.emit('open')

    expect(invalidate).toHaveBeenCalledTimes(2)
    unsubscribe()
  })

  it('creates no source without both account and tournament IDs', () => {
    const factory = vi.fn(() => new FakeSource())

    subscribeTournamentLive('', 'tour-two', vi.fn(), factory)()
    subscribeTournamentLive('user-two', '', vi.fn(), factory)()

    expect(factory).not.toHaveBeenCalled()
  })

  it('shares a target stream and survives a StrictMode-style cleanup and resubscribe', async () => {
    const source = new FakeSource()
    const factory = vi.fn(() => source)
    const first = subscribeTournamentLive('user-three', 'tour-three', vi.fn(), factory)
    const secondListener = vi.fn()

    first()
    const second = subscribeTournamentLive('user-three', 'tour-three', secondListener, factory)
    await Promise.resolve()
    source.emit('score')

    expect(factory).toHaveBeenCalledOnce()
    expect(source.close).not.toHaveBeenCalled()
    expect(secondListener).toHaveBeenCalledOnce()

    second()
    await Promise.resolve()
    expect(source.close).toHaveBeenCalledOnce()
  })

  it('closes the predecessor when the account and tournament target change', async () => {
    const oldSource = new FakeSource()
    const nextSource = new FakeSource()
    const sources = [oldSource, nextSource]
    const factory = vi.fn(() => {
      const source = sources.shift()
      if (!source) throw new Error('Unexpected live source creation')
      return source
    })
    const oldInvalidate = vi.fn()
    const nextInvalidate = vi.fn()
    const unsubscribeOld = subscribeTournamentLive('old-user', 'old-tour', oldInvalidate, factory)

    unsubscribeOld()
    const unsubscribeNext = subscribeTournamentLive('next-user', 'next-tour', nextInvalidate, factory)
    await Promise.resolve()
    oldSource.emit('score')
    nextSource.emit('score')

    expect(factory).toHaveBeenNthCalledWith(1, '/api/tournaments/old-tour/live', { withCredentials: true })
    expect(factory).toHaveBeenNthCalledWith(2, '/api/tournaments/next-tour/live', { withCredentials: true })
    expect(oldSource.close).toHaveBeenCalledOnce()
    expect(oldInvalidate).not.toHaveBeenCalled()
    expect(nextSource.close).not.toHaveBeenCalled()
    expect(nextInvalidate).toHaveBeenCalledOnce()

    unsubscribeNext()
    await Promise.resolve()
    expect(nextSource.close).toHaveBeenCalledOnce()
  })
})
