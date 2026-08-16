import { afterEach, describe, expect, it, vi } from 'vitest'
import { buildInvitationUrl, createOnboarding, decodeOnboardingResponse, type OnboardingRequest } from './onboarding'

const tournamentId = '00000000-0000-0000-0000-000000000010'
const playerId = '00000000-0000-0000-0000-000000000020'
const userId = '00000000-0000-0000-0000-000000000030'

const response = {
  tournament: {
    id: tournamentId,
    name: 'Høsttur',
    description: '',
    start_date: '2026-09-01',
    end_date: '2026-09-03',
    number_of_rounds: 1,
    status: 'draft',
    scoring_mode: 'individual',
    created_at: '2026-08-16T12:00:00Z',
    updated_at: '2026-08-16T12:00:00Z',
  },
  rounds: [{
    id: '00000000-0000-0000-0000-000000000040',
    tournament_id: tournamentId,
    round_number: 1,
    name: 'Åpningsrunde',
    round_date: '2026-09-01',
    course_id: null,
    course_name: '',
    tee_id: null,
    tee_name: '',
    number_of_holes: 18,
    status: 'draft',
    handicap_enabled: true,
    handicap_allowance_percent: 100,
    scoring_format: 'individual_stroke_play',
    created_at: '2026-08-16T12:00:00Z',
    updated_at: '2026-08-16T12:00:00Z',
  }],
  session: {
    user_id: userId,
    display_name: 'Morten',
    role: 'player',
    player_id: playerId,
    expires_at: '2026-08-17T12:00:00Z',
    csrf_token: 'csrf-token',
  },
  creator: { user_id: userId, player_id: playerId, tournament_role: 'admin' },
  invitation: {
    id: '00000000-0000-0000-0000-000000000050',
    token: 'A'.repeat(43),
    expires_at: '2026-09-10T00:00:00Z',
    max_uses: null,
  },
}

afterEach(() => vi.unstubAllGlobals())

describe('creator onboarding API', () => {
  it('decodes the complete response and validates linked identities', () => {
    expect(decodeOnboardingResponse(response)).toMatchObject({
      tournament: { id: tournamentId },
      creator: { player_id: playerId, tournament_role: 'admin' },
      invitation: { token: 'A'.repeat(43), max_uses: null },
    })
    expect(() => decodeOnboardingResponse({
      ...response,
      creator: { ...response.creator, player_id: '00000000-0000-0000-0000-000000000099' },
    })).toThrow('session_identity')
  })

  it('rejects malformed secrets, round ownership, and nullable fields', () => {
    expect(() => decodeOnboardingResponse({
      ...response,
      invitation: { ...response.invitation, token: 'raw-secret' },
    })).toThrow('invitation.token')
    expect(() => decodeOnboardingResponse({
      ...response,
      rounds: [{ ...response.rounds[0], tournament_id: '00000000-0000-0000-0000-000000000099' }],
    })).toThrow('tournament_id')
    expect(() => decodeOnboardingResponse({
      ...response,
      invitation: { ...response.invitation, max_uses: 20 },
    })).toThrow('invitation.max_uses')
  })

  it.each([
    ['non-draft tournament', { tournament: { ...response.tournament, status: 'active' } }, 'tournament.status'],
    ['invalid tournament range', { tournament: { ...response.tournament, end_date: '2026-08-31' } }, 'date_range'],
    ['wrong global role', { session: { ...response.session, role: 'admin' } }, 'session.role'],
    ['noncontiguous round', { rounds: [{ ...response.rounds[0], round_number: 2 }] }, 'round_number'],
    ['round outside dates', { rounds: [{ ...response.rounds[0], round_date: '2027-01-01' }] }, 'round_date'],
    ['configured course', { rounds: [{ ...response.rounds[0], course_id: '00000000-0000-0000-0000-000000000099' }] }, 'course_configuration'],
    ['wrong defaults', { rounds: [{ ...response.rounds[0], handicap_enabled: false }] }, 'defaults'],
    ['wrong summary mode', { tournament: { ...response.tournament, scoring_mode: 'team' } }, 'scoring_mode'],
  ])('rejects %s', (_label, override, path) => {
    expect(() => decodeOnboardingResponse({ ...response, ...override })).toThrow(path)
  })

  it('rejects duplicate round ids before caching the response', () => {
    const secondRound = { ...response.rounds[0], round_number: 2, name: 'Finale' }
    expect(() => decodeOnboardingResponse({
      ...response,
      tournament: { ...response.tournament, number_of_rounds: 2 },
      rounds: [response.rounds[0], secondRound],
    })).toThrow('rounds[1].id')
  })

  it('posts the exact nested request once', async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(response), {
      status: 201,
      headers: { 'content-type': 'application/json' },
    }))
    vi.stubGlobal('fetch', fetchMock)
    const input: OnboardingRequest = {
      creator: {
        account: { email: 'morten@example.no', password: 'et langt passord' },
        player: { display_name: 'Morten', handicap_index: 12.3 },
      },
      tournament: { name: 'Høsttur', description: '', start_date: '2026-09-01', end_date: '2026-09-03' },
      rounds: [{ round_number: 1, name: 'Åpningsrunde', round_date: '2026-09-01', scoring_format: 'individual_stroke_play' }],
    }

    await createOnboarding(input)

    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetchMock).toHaveBeenCalledWith('/api/onboarding/tournaments', {
      credentials: 'include',
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(input),
    })
  })

  it('builds an invitation with a path id and fragment-only token', () => {
    expect(buildInvitationUrl(
      'https://golf.example/app',
      response.invitation.id,
      response.invitation.token,
    )).toBe(`https://golf.example/join/${response.invitation.id}#token=${response.invitation.token}`)
  })
})
