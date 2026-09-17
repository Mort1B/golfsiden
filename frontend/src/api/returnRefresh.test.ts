import { QueryClient, QueryObserver } from '@tanstack/react-query'
import { describe, expect, it, vi } from 'vitest'
import { authKeys, type AuthSession } from './auth'
import { handleTournamentLiveSignal } from './liveInvalidation'
import { scoringKeys } from './scorecards'
import { session } from '../features/tournaments/lifecycle/__tests__/fixtures'

function deferred<T>() {
  let resolve: (value: T) => void = () => undefined
  let reject: (reason: Error) => void = () => undefined
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
function fixture() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } })
  const authRead = vi.fn<() => Promise<AuthSession | null>>().mockResolvedValue(session)
  const cardRead = vi.fn<() => Promise<string>>().mockResolvedValue('locked')
  const key = scoringKeys.scoring(session.user_id, 'round', { type: 'player', id: 'player' })
  const auth = new QueryObserver(client, { queryKey: authKeys.session, queryFn: authRead, initialData: session })
  const card = new QueryObserver(client, { queryKey: key, queryFn: cardRead, initialData: 'cached open' })
  const stopAuth = auth.subscribe(() => undefined), stopCard = card.subscribe(() => undefined)
  return { client, authRead, cardRead, key, resume: () => handleTournamentLiveSignal(client, session.user_id, 'resume'),
    close: () => { stopAuth(); stopCard(); client.clear() } }
}

describe('overlapping page returns', () => {
  it('coalesces a synchronous burst and completes one fresh follow-up after an in-flight private read', async () => {
    const f = fixture(), oldRead = deferred<string>()
    f.cardRead.mockReturnValueOnce(oldRead.promise)
    try {
      const work = f.resume()
      for (let i = 0; i < 20; i++) expect(f.resume()).toBe(work)
      await vi.waitFor(() => expect(f.cardRead).toHaveBeenCalledTimes(1))
      expect(f.authRead).toHaveBeenCalledTimes(1)
      for (let i = 0; i < 20; i++) expect(f.resume()).toBe(work)
      expect(f.authRead).toHaveBeenCalledTimes(1)
      oldRead.resolve('captured open')
      await work
      expect(f.authRead).toHaveBeenCalledTimes(2)
      expect(f.cardRead).toHaveBeenCalledTimes(2)
      expect(f.client.getQueryData(f.key)).toBe('locked')
    } finally { oldRead.resolve('cleanup'); f.close() }
  })

  it('retains a new return during the follow-up without polling or starting parallel passes', async () => {
    const f = fixture(), first = deferred<string>(), second = deferred<string>()
    f.cardRead.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise)
    try {
      const work = f.resume()
      await vi.waitFor(() => expect(f.cardRead).toHaveBeenCalledTimes(1))
      f.resume(); first.resolve('first')
      await vi.waitFor(() => expect(f.cardRead).toHaveBeenCalledTimes(2))
      for (let i = 0; i < 20; i++) expect(f.resume()).toBe(work)
      expect(f.authRead).toHaveBeenCalledTimes(2)
      second.resolve('second'); await work
      expect(f.authRead).toHaveBeenCalledTimes(3)
      expect(f.cardRead).toHaveBeenCalledTimes(3)
      await f.resume()
      expect(f.authRead).toHaveBeenCalledTimes(4)
      expect(f.cardRead).toHaveBeenCalledTimes(4)
    } finally { first.resolve('cleanup'); second.resolve('cleanup'); f.close() }
  })

  it('revalidates again when another return arrives while session validation is pending', async () => {
    const f = fixture(), pendingAuth = deferred<AuthSession | null>()
    f.authRead.mockReturnValueOnce(pendingAuth.promise)
    try {
      const work = f.resume()
      await vi.waitFor(() => expect(f.authRead).toHaveBeenCalledTimes(1))
      expect(f.cardRead).not.toHaveBeenCalled()
      expect(f.resume()).toBe(work)
      pendingAuth.resolve(session); await work
      expect(f.authRead).toHaveBeenCalledTimes(2)
      expect(f.cardRead).toHaveBeenCalledTimes(2)
    } finally { pendingAuth.resolve(null); f.close() }
  })

  it.each(['expired', 'changed', 'failed'] as const)('does not refresh previous private data when follow-up authentication is %s', async outcome => {
    const f = fixture(), first = deferred<string>()
    f.cardRead.mockReturnValueOnce(first.promise)
    try {
      const work = f.resume()
      await vi.waitFor(() => expect(f.cardRead).toHaveBeenCalledTimes(1))
      if (outcome === 'failed') f.authRead.mockRejectedValueOnce(new Error('network unavailable'))
      else f.authRead.mockResolvedValueOnce(outcome === 'expired' ? null : { ...session, user_id: 'other-user' })
      f.resume(); first.resolve('captured'); await work
      expect(f.authRead).toHaveBeenCalledTimes(2)
      expect(f.cardRead).toHaveBeenCalledTimes(1)
      // A subsequent authorized return is recoverable; the failed batch is not stuck.
      if (outcome !== 'failed') f.client.setQueryData(authKeys.session, session)
      await f.resume()
      expect(f.cardRead).toHaveBeenCalledTimes(2)
    } finally { first.resolve('cleanup'); f.close() }
  })

  it('abandons an old user batch if identity changes while a private read is pending', async () => {
    const f = fixture(), first = deferred<string>(), newAuth = deferred<AuthSession | null>()
    const otherSession = { ...session, user_id: 'other-user' }
    f.cardRead.mockReturnValueOnce(first.promise)
    try {
      const work = f.resume()
      await vi.waitFor(() => expect(f.cardRead).toHaveBeenCalledTimes(1))
      f.resume()
      f.client.setQueryData(authKeys.session, otherSession)
      f.authRead.mockReturnValueOnce(newAuth.promise)
      const newWork = f.client.invalidateQueries({ queryKey: authKeys.session, exact: true })
      await vi.waitFor(() => expect(f.authRead).toHaveBeenCalledTimes(2))
      first.resolve('captured'); await work
      expect(f.authRead).toHaveBeenCalledTimes(2)
      expect(f.cardRead).toHaveBeenCalledTimes(1)
      newAuth.resolve(otherSession); await newWork
      expect(f.client.getQueryData(authKeys.session)).toEqual(otherSession)
    } finally { first.resolve('cleanup'); newAuth.resolve(null); f.close() }
  })

  it('uses the queued return after a failed private read, then stops without automatic retries', async () => {
    const f = fixture(), first = deferred<string>()
    f.cardRead.mockReturnValueOnce(first.promise).mockRejectedValueOnce(new Error('still offline'))
    try {
      const work = f.resume()
      await vi.waitFor(() => expect(f.cardRead).toHaveBeenCalledTimes(1))
      f.resume(); first.reject(new Error('offline')); await work
      expect(f.authRead).toHaveBeenCalledTimes(2)
      expect(f.cardRead).toHaveBeenCalledTimes(2)
      expect(f.client.getQueryState(f.key)?.status).toBe('error')
      await f.resume()
      expect(f.client.getQueryData(f.key)).toBe('locked')
    } finally { first.resolve('cleanup'); f.close() }
  })
})
