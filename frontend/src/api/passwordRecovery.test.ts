import { afterEach, expect, it, vi } from 'vitest'
import { decodeRecoveryPreview, decodeRecoveryReceipt, passwordRecoveryApi } from './passwordRecovery'
const id = '00000000-0000-0000-0000-000000000001'
const token = 'a'.repeat(43)
const metadata = { id, expires_at: '2099-01-01T12:00:00Z' }
const receipt = { ...metadata, reset_url: `https://golf.example/reset-password/${id}#token=${token}` }
afterEach(() => vi.unstubAllGlobals())
it('validates public identity and safe exact recovery links without echoing secrets', () => {
  expect(decodeRecoveryReceipt(receipt)).toEqual(receipt)
  expect(decodeRecoveryReceipt({ ...receipt, reset_url: receipt.reset_url.replace('https://golf.example', 'http://127.0.0.1:5173') })).toHaveProperty('id', id)
  expect(() => decodeRecoveryPreview(metadata, 'different')).toThrow('recovery.id')
  expect(() => decodeRecoveryPreview({ ...metadata, expires_at: '2099-99-99T00:00:00Z' })).toThrow('recovery.expires_at')
  for (const reset_url of [receipt.reset_url.replace('https:', 'javascript:'), receipt.reset_url.replace('https:', 'http:'), receipt.reset_url.replace('golf.example', 'u:p@golf.example'), receipt.reset_url.replace('#token=', '?secret=x#token='), receipt.reset_url.replace(id, 'other'), receipt.reset_url + '&x=y', receipt.reset_url.replace('/reset-password/', '/join/'), receipt.reset_url.replace('#token=', '#other=')]) {
    try { decodeRecoveryReceipt({ ...receipt, reset_url }); throw new Error('unexpected success') }
    catch (error) { expect(error).toBeInstanceOf(Error); expect(String(error)).toContain('recovery.reset_url'); expect(String(error)).not.toContain(token) }
  }
})
it('sends secrets only in POST bodies with CSRF only for administrator actions', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(new Response(JSON.stringify(receipt), { status: 201 }))
    .mockResolvedValueOnce(new Response(null, { status: 204 }))
    .mockResolvedValueOnce(new Response(JSON.stringify(metadata)))
    .mockResolvedValueOnce(new Response(null, { status: 204 }))
  vi.stubGlobal('fetch', fetcher)
  await passwordRecoveryApi.issue(id, id, 'current password', 'csrf')
  await passwordRecoveryApi.revoke(id, id, 'current password', 'csrf')
  await passwordRecoveryApi.preview(id, token)
  await passwordRecoveryApi.redeem(id, token, ' spaced password ', ' spaced password ')
  const calls = fetcher.mock.calls
  expect(calls[0]).toEqual([`/api/tournaments/${id}/players/${id}/password-recovery`, expect.objectContaining({ credentials: 'include', method: 'POST', cache: 'no-store', referrerPolicy: 'no-referrer', headers: { 'content-type': 'application/json', 'x-csrf-token': 'csrf' }, body: JSON.stringify({ current_password: 'current password' }) })])
  expect(calls[1]?.[0]).toContain('/revoke')
  expect(calls[2]).toEqual([`/api/auth/password-recovery/${id}/preview`, expect.objectContaining({ headers: { 'content-type': 'application/json' }, body: JSON.stringify({ token }) })])
  expect(calls[3]?.[1]).toHaveProperty('body', JSON.stringify({ token, new_password: ' spaced password ', confirm_password: ' spaced password ' }))
  for (const call of calls) expect(call[0]).not.toContain(token)
})
