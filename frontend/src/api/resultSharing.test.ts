import { afterEach, expect, it, vi } from 'vitest'
import { resultSharingApi, resultShareKey } from './resultSharing'
import { decodePublicResults, decodeShareReceipt, decodeShareStatus } from './resultSharing/decoders'
import { publicFixture, shareGrant, shareId, shareSecret } from './resultSharing/__tests__/fixtures'
import { requestDecoded, requestNoContent } from './http'
const trip = '00000000-0000-0000-0000-000000008001'
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.useRealTimers() })
it('decodes minimal gross/net standings and rejects the other metric or grant identity', () => {
  for (const metric of ['gross', 'net'] as const) expect(decodePublicResults(publicFixture(metric), shareId, metric)).toEqual(publicFixture(metric))
  expect(() => decodePublicResults(publicFixture(), shareId, 'net')).toThrow('identity')
  expect(() => decodePublicResults(publicFixture(), trip, 'gross')).toThrow('identity')
})
it('rejects private fields, nested visibility extras and missing public fields', () => {
  const board = publicFixture()
  expect(() => decodePublicResults({ ...board, player_id: trip }, shareId, 'gross')).toThrow()
  expect(() => decodePublicResults({ ...board, entries: board.entries.map(entry => ({ ...entry, net_total: 1 })) }, shareId, 'gross')).toThrow()
  expect(() => decodePublicResults({ ...board, visibility: { mode: 'full', administrator: true } }, shareId, 'gross')).toThrow()
  expect(() => decodePublicResults({ ...board, tie_break_policy: undefined }, shareId, 'gross')).toThrow()
})
it('rejects incoherent rank, qualification, total, provisional and tie metadata', () => {
  const board = publicFixture()
  for (const fields of [{ total: 999 }, { provisional: true }, { provisional_holes_scored: 19 }, { eligible: false }, { position: null }, { tie_break_score_to_par: null }]) {
    expect(() => decodePublicResults({ ...board, entries: board.entries.map((entry, index) => index === 0 ? { ...entry, ...fields } : entry) }, shareId, 'gross')).toThrow()
  }
  expect(() => decodePublicResults({ ...board, visibility: { mode: 'front_nine' } }, shareId, 'gross')).toThrow()
})
it('keeps metadata secret-free and validates one-time issuance receipts', () => {
  expect(decodeShareStatus({ tournament_id: trip, grant: null }, trip)).toEqual({ tournament_id: trip, grant: null })
  expect(decodeShareReceipt({ tournament_id: trip, grant: shareGrant, token: shareSecret }, trip).token).toBe(shareSecret)
  expect(() => decodeShareStatus({ tournament_id: trip, grant: shareGrant, token: shareSecret }, trip)).toThrow()
  expect(() => decodeShareReceipt({ tournament_id: trip, grant: { ...shareGrant, revoked_at: shareGrant.created_at }, token: shareSecret }, trip)).toThrow()
  expect(() => decodeShareStatus({ tournament_id: shareId, grant: shareGrant }, trip)).toThrow('identity')
  expect(JSON.stringify(resultShareKey('user', trip))).not.toContain(shareSecret)
})
it('omits cookies on public POST, confines the token to the body, and keeps private cookie defaults', async () => {
  const fetcher = vi.fn(async () => new Response(JSON.stringify(publicFixture())))
  vi.stubGlobal('fetch', fetcher)
  await resultSharingApi.read(shareId, shareSecret, 'gross', new AbortController().signal)
  expect(fetcher).toHaveBeenCalledWith(`/api/public/results/${shareId}`, expect.objectContaining({
    credentials: 'omit', cache: 'no-store', referrerPolicy: 'no-referrer', body: JSON.stringify({ token: shareSecret, metric: 'gross' }),
  }))
  await requestDecoded('/private', value => value)
  expect(fetcher).toHaveBeenLastCalledWith('/private', { credentials: 'include' })
  fetcher.mockImplementation(async () => new Response(null, { status: 204 }))
  await requestNoContent('/public', { credentials: 'omit' })
  expect(fetcher).toHaveBeenLastCalledWith('/public', { credentials: 'omit' })
})
it('sanitizes errors including server reflections and aborts timed-out public requests', async () => {
  vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ error: { code: 'unexpected', message: shareSecret } }), { status: 404 })))
  await expect(resultSharingApi.read(shareId, shareSecret, 'gross', new AbortController().signal)).rejects.toMatchObject({ status: 404, message: 'Resultatlenken er ikke tilgjengelig.' })
  vi.useFakeTimers()
  vi.stubGlobal('fetch', vi.fn((_path: string, init: RequestInit) => new Promise((_resolve, reject) => { init.signal?.addEventListener('abort', () => reject(new Error(shareSecret))) })))
  const request = resultSharingApi.read(shareId, shareSecret, 'gross', new AbortController().signal)
  const assertion = expect(request).rejects.toThrow('Kunne ikke hente delte resultater. Prøv igjen.')
  await vi.advanceTimersByTimeAsync(12_000)
  await assertion
})
