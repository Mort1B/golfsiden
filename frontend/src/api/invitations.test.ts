import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  acceptInvitation,
  buildInvitationUrl,
  decodeInvitationAcceptance,
  decodeInvitationMetadata,
  decodeInvitationPreview,
  decodeInvitationRegistration,
  decodeInvitationSecret,
  invitationKeys,
  issueInvitation,
  listInvitations,
  previewInvitation,
  registerInvitation,
  revokeInvitation,
  rotateInvitation,
} from './invitations'

const invitationId = '00000000-0000-0000-0000-000000000001'
const tournamentId = '00000000-0000-0000-0000-000000000002'
const seriesId = '00000000-0000-0000-0000-000000000003'
const userId = '00000000-0000-0000-0000-000000000004'
const playerId = '00000000-0000-0000-0000-000000000005'
const token = 'A'.repeat(43)

const metadata = {
  id: invitationId,
  tournament_id: tournamentId,
  series_id: seriesId,
  predecessor_id: null,
  created_by_user_id: userId,
  created_at: '2026-08-16T12:00:00Z',
  expires_at: '2026-09-16T12:00:00Z',
  revoked_at: null,
  revoked_by_user_id: null,
  revocation_actor_known: false,
  max_uses: 10,
  redemption_count: 2,
}

const preview = {
  tournament: { id: tournamentId, name: 'Høsttur', start_date: '2026-09-01', end_date: '2026-09-04' },
  invitation: { expires_at: '2026-09-16T12:00:00Z' },
}

const session = {
  user_id: userId,
  display_name: 'Morten',
  role: 'player',
  player_id: playerId,
  expires_at: '2026-08-17T12:00:00Z',
  csrf_token: 'csrf-value',
}

afterEach(() => vi.unstubAllGlobals())

describe('invitation decoders', () => {
  it('strictly decodes preview and finite join statuses', () => {
    expect(decodeInvitationPreview(preview)).toEqual(preview)
    expect(decodeInvitationRegistration({ status: 'joined', tournament_id: tournamentId, player_id: playerId, session })).toMatchObject({ status: 'joined', player_id: playerId })
    expect(decodeInvitationAcceptance({ status: 'already_joined', tournament_id: tournamentId, player_id: playerId })).toEqual({ status: 'already_joined', tournament_id: tournamentId, player_id: playerId })
    expect(() => decodeInvitationAcceptance({ status: 'pending', tournament_id: tournamentId, player_id: playerId })).toThrow('acceptance.status')
    expect(() => decodeInvitationRegistration({ status: 'joined', tournament_id: tournamentId, player_id: invitationId, session })).toThrow('session.player_id')
    expect(() => decodeInvitationPreview({ ...preview, tournament: { ...preview.tournament, end_date: '2026-08-31' } })).toThrow('date_range')
  })

  it('strictly decodes admin metadata and one-time secrets', () => {
    expect(decodeInvitationMetadata(metadata)).toEqual(metadata)
    expect(decodeInvitationSecret({ ...metadata, token })).toEqual({ ...metadata, token })
    expect(() => decodeInvitationMetadata({ ...metadata, max_uses: 0 })).toThrow('max_uses')
    expect(() => decodeInvitationMetadata({ ...metadata, redemption_count: -1 })).toThrow('redemption_count')
    expect(() => decodeInvitationMetadata({ ...metadata, revocation_actor_known: 'false' })).toThrow('revocation_actor_known')
    expect(() => decodeInvitationSecret({ ...metadata, token: 'raw-secret' })).toThrow('invitation.token')
  })
})

describe('invitation request secrecy', () => {
  it('keeps the secret out of query keys and preview request URLs', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      void input
      void init
      return new Response(JSON.stringify(preview), { status: 200 })
    })
    vi.stubGlobal('fetch', fetchMock)

    await previewInvitation(invitationId, token)

    expect(invitationKeys.preview(invitationId)).toEqual(['invitations', invitationId, 'preview'])
    expect(invitationKeys.list(tournamentId)).toEqual(['tournaments', tournamentId, 'invitations'])
    const [requestUrl, requestInit] = fetchMock.mock.calls[0] ?? []
    expect(requestUrl).toBe(`/api/invitations/${invitationId}/preview`)
    expect(String(requestUrl)).not.toContain(token)
    expect(requestInit?.body).toBe(JSON.stringify({ token }))
  })

  it('posts registration and acceptance secrets only in JSON bodies', async () => {
    const registration = { status: 'joined', tournament_id: tournamentId, player_id: playerId, session }
    const acceptance = { status: 'joined', tournament_id: tournamentId, player_id: playerId }
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(registration), { status: 201 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(acceptance), { status: 201 }))
    vi.stubGlobal('fetch', fetchMock)
    const input = {
      account: { email: 'morten@example.no', password: 'tolv tegn eller mer' },
      player: { display_name: 'Morten', handicap_index: 12.3 },
    }

    await registerInvitation(invitationId, token, input)
    await acceptInvitation(invitationId, token, 'csrf-value')

    expect(fetchMock.mock.calls[0]?.[0]).toBe(`/api/invitations/${invitationId}/register`)
    expect(fetchMock.mock.calls[0]?.[1]?.body).toBe(JSON.stringify({ token, ...input }))
    expect(fetchMock.mock.calls[1]?.[0]).toBe(`/api/invitations/${invitationId}/accept`)
    expect(fetchMock.mock.calls[1]?.[1]).toMatchObject({
      headers: { 'content-type': 'application/json', 'x-csrf-token': 'csrf-value' },
      body: JSON.stringify({ token }),
    })
    expect(fetchMock.mock.calls.flatMap((call) => [String(call[0])]).join(' ')).not.toContain(token)
  })

  it('places a share secret in the fragment, never the path or query', () => {
    const result = new URL(buildInvitationUrl('https://golf.example/app', invitationId, token))
    expect(result.pathname).toBe(`/join/${invitationId}`)
    expect(result.search).toBe('')
    expect(result.hash).toBe(`#token=${token}`)
  })
})

describe('invitation administration requests', () => {
  it('decodes list, issue, rotate, and validates a 204 revoke', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify([metadata]), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...metadata, token }), { status: 201 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...metadata, id: playerId, predecessor_id: invitationId, token }), { status: 201 }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(listInvitations(tournamentId)).resolves.toEqual([metadata])
    await expect(issueInvitation(tournamentId, { expires_at: metadata.expires_at, max_uses: 10 }, 'csrf')).resolves.toMatchObject({ token })
    await expect(rotateInvitation(tournamentId, invitationId, 'csrf')).resolves.toMatchObject({ predecessor_id: invitationId, token })
    await expect(revokeInvitation(tournamentId, invitationId, 'csrf')).resolves.toBeUndefined()

    expect(fetchMock.mock.calls[1]?.[1]?.body).toBe(JSON.stringify({ expires_at: metadata.expires_at, max_uses: 10 }))
    expect(fetchMock.mock.calls[2]?.[1]?.body).toBe('{}')
    expect(fetchMock.mock.calls[3]?.[1]).toMatchObject({ method: 'DELETE', headers: { 'x-csrf-token': 'csrf' } })
  })
})
