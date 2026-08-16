import { decodeArray, decodeObject, decodeString, decodeTimestamp, decodeUuid, invalidData } from './decoder'
import { decodeAuthSession, type AuthSession } from './auth'
import { jsonRequest, requestDecoded } from './http'
import { decodeRound, decodeTournament } from './tournaments'
import type { Round, ScoringFormat, Tournament } from './types'

export interface OnboardingRequest {
  creator: {
    account: { username: string; password: string }
    player: { display_name: string; handicap_index: number }
  }
  tournament: {
    name: string
    description: string
    start_date: string
    end_date: string
  }
  rounds: Array<{
    round_number: number
    name: string
    round_date: string
    scoring_format: ScoringFormat
  }>
}

export interface OnboardingResponse {
  tournament: Tournament
  rounds: Round[]
  session: AuthSession
  creator: {
    user_id: string
    player_id: string
    tournament_role: 'admin'
  }
  invitation: {
    id: string
    token: string
    expires_at: string
    max_uses: null
  }
}

function validateCreatedDefaults(tournament: Tournament, rounds: Round[], session: AuthSession): void {
  if (tournament.status !== 'draft') invalidData('opprettingsdata', 'onboarding.tournament.status')
  if (tournament.end_date < tournament.start_date) invalidData('opprettingsdata', 'onboarding.tournament.date_range')
  if (session.role !== 'player') invalidData('opprettingsdata', 'onboarding.session.role')
  const roundIds = new Set<string>()
  let hasIndividual = false
  let hasTeam = false
  rounds.forEach((round, index) => {
    const path = `onboarding.rounds[${index}]`
    if (round.id === '' || roundIds.has(round.id)) invalidData('opprettingsdata', `${path}.id`)
    roundIds.add(round.id)
    if (round.tournament_id !== tournament.id) invalidData('opprettingsdata', `${path}.tournament_id`)
    if (round.round_number !== index + 1) invalidData('opprettingsdata', `${path}.round_number`)
    if (round.round_date < tournament.start_date || round.round_date > tournament.end_date) {
      invalidData('opprettingsdata', `${path}.round_date`)
    }
    if (round.status !== 'draft') invalidData('opprettingsdata', `${path}.status`)
    if (round.number_of_holes !== 18 || !round.handicap_enabled || round.handicap_allowance_percent !== 100) {
      invalidData('opprettingsdata', `${path}.defaults`)
    }
    if (round.course_id !== null || round.tee_id !== null || round.course_name !== '' || round.tee_name !== '') {
      invalidData('opprettingsdata', `${path}.course_configuration`)
    }
    hasIndividual ||= round.scoring_format === 'individual_stroke_play'
    hasTeam ||= round.scoring_format === 'team_scramble'
  })
  const derivedMode = hasIndividual && hasTeam ? 'combined' : hasTeam ? 'team' : 'individual'
  if (tournament.scoring_mode !== derivedMode) invalidData('opprettingsdata', 'onboarding.tournament.scoring_mode')
}

export function decodeOnboardingResponse(value: unknown): OnboardingResponse {
  const data = decodeObject(value, 'onboarding', 'opprettingsdata')
  const creator = decodeObject(data.creator, 'onboarding.creator', 'opprettingsdata')
  const invitation = decodeObject(data.invitation, 'onboarding.invitation', 'opprettingsdata')
  if (creator.tournament_role !== 'admin') invalidData('opprettingsdata', 'onboarding.creator.tournament_role')
  const token = decodeString(invitation.token, 'onboarding.invitation.token', 'opprettingsdata')
  if (!/^[A-Za-z0-9_-]{43}$/.test(token)) invalidData('opprettingsdata', 'onboarding.invitation.token')
  if (invitation.max_uses !== null) invalidData('opprettingsdata', 'onboarding.invitation.max_uses')
  const rounds = decodeArray(data.rounds, 'onboarding.rounds', decodeRound, 'rundedata')
  const tournament = decodeTournament(data.tournament, 'onboarding.tournament')
  if (rounds.length !== tournament.number_of_rounds) {
    invalidData('opprettingsdata', 'onboarding.rounds.length')
  }
  const session = decodeAuthSession(data.session)
  const userId = decodeUuid(creator.user_id, 'onboarding.creator.user_id', 'opprettingsdata')
  const playerId = decodeUuid(creator.player_id, 'onboarding.creator.player_id', 'opprettingsdata')
  if (session.user_id !== userId || session.player_id !== playerId) {
    invalidData('opprettingsdata', 'onboarding.creator.session_identity')
  }
  validateCreatedDefaults(tournament, rounds, session)
  return {
    tournament,
    rounds,
    session,
    creator: {
      user_id: userId,
      player_id: playerId,
      tournament_role: 'admin',
    },
    invitation: {
      id: decodeUuid(invitation.id, 'onboarding.invitation.id', 'opprettingsdata'),
      token,
      expires_at: decodeTimestamp(invitation.expires_at, 'onboarding.invitation.expires_at', 'opprettingsdata'),
      max_uses: null,
    },
  }
}

export function createOnboarding(input: OnboardingRequest): Promise<OnboardingResponse> {
  return requestDecoded('/api/onboarding/tournaments', decodeOnboardingResponse, jsonRequest('POST', input))
}

export function buildInvitationUrl(origin: string, invitationId: string, token: string): string {
  const url = new URL(`/join/${invitationId}`, origin)
  url.hash = new URLSearchParams({ token }).toString()
  return url.toString()
}
