import { describe, expect, it, vi, afterEach } from 'vitest'
import { decodeClaimPreview, decodeClaimReceipt, decodeClaimRegistration, decodePlayerAccounts, playerClaimsApi } from './playerClaims'
import { session, tournament } from '../features/tournaments/lifecycle/__tests__/fixtures'
const player = '00000000-0000-0000-0000-000000000010'
const claim = '00000000-0000-0000-0000-000000000011'
const receipt = { player_id: player, claim_id: claim, expires_at: '2099-01-01T12:00:00Z', token: 'a'.repeat(43) }
const preview = { tournament: { id: tournament.id, name: 'Trip' }, player: { id: player, display_name: 'Prepared' }, expires_at: receipt.expires_at }
afterEach(() => vi.unstubAllGlobals())
describe('claim transport boundaries', () => {
  it('validates identifiers, real timestamps, tokens and expected player', () => {
    expect(decodeClaimReceipt(receipt, player)).toEqual(receipt)
    expect(() => decodeClaimReceipt(receipt, claim)).toThrow()
    expect(() => decodeClaimReceipt({ ...receipt, token: 'short' })).toThrow()
    expect(() => decodeClaimReceipt({ ...receipt, expires_at: '2099-99-01T12:00:00Z' })).toThrow()
    expect(() => decodeClaimPreview({ ...preview, player: { ...preview.player, id: 'bad' } })).toThrow()
  })
  it('cross-checks registration identity against preview and session', () => {
    const result = { tournament_id: tournament.id, player_id: player, session: { ...session, player_id: player } }
    expect(decodeClaimRegistration(result, preview)).toEqual(result)
    expect(() => decodeClaimRegistration({ ...result, tournament_id: claim }, preview)).toThrow()
    expect(() => decodeClaimRegistration({ ...result, session }, preview)).toThrow()
    expect(() => decodeClaimRegistration({ ...result, player_id: claim }, preview)).toThrow()
  })
  it('requires exact metadata nullability and unique player identities', () => {
    const account = { player_id: player, has_account: false, claim_id: null, claim_expires_at: null, claim_revoked_at: null, claimed_at: null }
    expect(decodePlayerAccounts([account])).toEqual([account])
    expect(() => decodePlayerAccounts([{ ...account, has_account: 'false' }])).toThrow()
    expect(() => decodePlayerAccounts([{ ...account, claimed_at: undefined }])).toThrow()
    expect(() => decodePlayerAccounts([account, account])).toThrow()
  })
  it('sends tokens only in POST bodies without caching or referrers', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(preview)))
    vi.stubGlobal('fetch', fetch)
    await playerClaimsApi.preview(claim, receipt.token)
    expect(fetch).toHaveBeenCalledWith(`/api/player-claims/${claim}/preview`, expect.objectContaining({ method: 'POST', body: JSON.stringify({ token: receipt.token }), cache: 'no-store', referrerPolicy: 'no-referrer' }))
  })
})
