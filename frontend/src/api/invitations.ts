import { decodeAuthSession, type AuthSession } from './auth'
import {
  decodeArray,
  decodeBoolean,
  decodeDate,
  decodeInteger,
  decodeObject,
  decodeString,
  decodeTimestamp,
  decodeUuid,
  invalidData,
} from './decoder'
import { jsonRequest, requestDecoded, requestNoContent } from './http'

export interface InvitationPreview {
  tournament: {
    id: string
    name: string
    start_date: string
    end_date: string
  }
  invitation: { expires_at: string }
}

export interface InvitationRegistrationInput {
  account: { username: string; password: string }
  player: { display_name: string; handicap_index: number }
}

export interface InvitationRegistrationResult {
  status: 'joined'
  tournament_id: string
  player_id: string
  session: AuthSession
}

export interface InvitationAcceptanceResult {
  status: 'joined' | 'already_joined'
  tournament_id: string
  player_id: string
}

export interface InvitationMetadata {
  id: string
  tournament_id: string
  series_id: string
  predecessor_id: string | null
  created_by_user_id: string
  created_at: string
  expires_at: string
  revoked_at: string | null
  revoked_by_user_id: string | null
  revocation_actor_known: boolean
  max_uses: number | null
  redemption_count: number
}

export interface InvitationSecretResult extends InvitationMetadata {
  token: string
}

export interface InvitationIssueInput {
  expires_at: string
  max_uses: number | null
}

const tokenPattern = /^[A-Za-z0-9_-]{43}$/

function decodeToken(value: unknown, path: string): string {
  const token = decodeString(value, path, 'invitasjonsdata')
  if (!tokenPattern.test(token)) invalidData('invitasjonsdata', path)
  return token
}

function joinedStatus(value: unknown, path: string): 'joined' {
  if (value !== 'joined') invalidData('invitasjonsdata', path)
  return value
}

function acceptanceStatus(value: unknown, path: string): 'joined' | 'already_joined' {
  if (value !== 'joined' && value !== 'already_joined') invalidData('invitasjonsdata', path)
  return value
}

export function decodeInvitationPreview(value: unknown): InvitationPreview {
  const data = decodeObject(value, 'preview', 'invitasjonsdata')
  const tournament = decodeObject(data.tournament, 'preview.tournament', 'invitasjonsdata')
  const invitation = decodeObject(data.invitation, 'preview.invitation', 'invitasjonsdata')
  const startDate = decodeDate(tournament.start_date, 'preview.tournament.start_date', 'invitasjonsdata')
  const endDate = decodeDate(tournament.end_date, 'preview.tournament.end_date', 'invitasjonsdata')
  if (endDate < startDate) invalidData('invitasjonsdata', 'preview.tournament.date_range')
  return {
    tournament: {
      id: decodeUuid(tournament.id, 'preview.tournament.id', 'invitasjonsdata'),
      name: decodeString(tournament.name, 'preview.tournament.name', 'invitasjonsdata'),
      start_date: startDate,
      end_date: endDate,
    },
    invitation: {
      expires_at: decodeTimestamp(invitation.expires_at, 'preview.invitation.expires_at', 'invitasjonsdata'),
    },
  }
}

export function decodeInvitationRegistration(value: unknown): InvitationRegistrationResult {
  const data = decodeObject(value, 'registration', 'invitasjonsdata')
  const result: InvitationRegistrationResult = {
    status: joinedStatus(data.status, 'registration.status'),
    tournament_id: decodeUuid(data.tournament_id, 'registration.tournament_id', 'invitasjonsdata'),
    player_id: decodeUuid(data.player_id, 'registration.player_id', 'invitasjonsdata'),
    session: decodeAuthSession(data.session),
  }
  if (result.session.player_id !== result.player_id) {
    invalidData('invitasjonsdata', 'registration.session.player_id')
  }
  return result
}

export function decodeInvitationAcceptance(value: unknown): InvitationAcceptanceResult {
  const data = decodeObject(value, 'acceptance', 'invitasjonsdata')
  return {
    status: acceptanceStatus(data.status, 'acceptance.status'),
    tournament_id: decodeUuid(data.tournament_id, 'acceptance.tournament_id', 'invitasjonsdata'),
    player_id: decodeUuid(data.player_id, 'acceptance.player_id', 'invitasjonsdata'),
  }
}

export function decodeInvitationMetadata(value: unknown, path = 'invitation'): InvitationMetadata {
  const data = decodeObject(value, path, 'invitasjonsdata')
  return {
    id: decodeUuid(data.id, `${path}.id`, 'invitasjonsdata'),
    tournament_id: decodeUuid(data.tournament_id, `${path}.tournament_id`, 'invitasjonsdata'),
    series_id: decodeUuid(data.series_id, `${path}.series_id`, 'invitasjonsdata'),
    predecessor_id: data.predecessor_id === null
      ? null
      : decodeUuid(data.predecessor_id, `${path}.predecessor_id`, 'invitasjonsdata'),
    created_by_user_id: decodeUuid(data.created_by_user_id, `${path}.created_by_user_id`, 'invitasjonsdata'),
    created_at: decodeTimestamp(data.created_at, `${path}.created_at`, 'invitasjonsdata'),
    expires_at: decodeTimestamp(data.expires_at, `${path}.expires_at`, 'invitasjonsdata'),
    revoked_at: data.revoked_at === null
      ? null
      : decodeTimestamp(data.revoked_at, `${path}.revoked_at`, 'invitasjonsdata'),
    revoked_by_user_id: data.revoked_by_user_id === null
      ? null
      : decodeUuid(data.revoked_by_user_id, `${path}.revoked_by_user_id`, 'invitasjonsdata'),
    revocation_actor_known: decodeBoolean(data.revocation_actor_known, `${path}.revocation_actor_known`, 'invitasjonsdata'),
    max_uses: data.max_uses === null
      ? null
      : decodeInteger(data.max_uses, `${path}.max_uses`, 1, undefined, 'invitasjonsdata'),
    redemption_count: decodeInteger(data.redemption_count, `${path}.redemption_count`, 0, undefined, 'invitasjonsdata'),
  }
}

export function decodeInvitationSecret(value: unknown): InvitationSecretResult {
  const data = decodeObject(value, 'invitation', 'invitasjonsdata')
  return {
    ...decodeInvitationMetadata(value),
    token: decodeToken(data.token, 'invitation.token'),
  }
}

export function decodeExpectedInvitationList(value: unknown, expectedTournamentId: string): InvitationMetadata[] {
  const invitations = decodeArray(value, 'invitations', decodeInvitationMetadata, 'invitasjonsdata')
  const invitationIds = new Set<string>()
  invitations.forEach((invitation, index) => {
    if (invitation.tournament_id !== expectedTournamentId) {
      invalidData('invitasjonsdata', `invitations[${index}].tournament_id identity`)
    }
    if (invitationIds.has(invitation.id)) invalidData('invitasjonsdata', `invitations[${index}].id duplicate`)
    invitationIds.add(invitation.id)
  })
  return invitations
}

export function decodeExpectedInvitationSecret(
  value: unknown,
  expectedTournamentId: string,
  expectedPredecessorId?: string,
): InvitationSecretResult {
  const invitation = decodeInvitationSecret(value)
  if (invitation.tournament_id !== expectedTournamentId) {
    invalidData('invitasjonsdata', 'invitation.tournament_id identity')
  }
  if (expectedPredecessorId !== undefined && invitation.predecessor_id !== expectedPredecessorId) {
    invalidData('invitasjonsdata', 'invitation.predecessor_id identity')
  }
  return invitation
}

export const invitationKeys = {
  preview: (invitationId: string) => ['invitations', invitationId, 'preview'] as const,
  list: (tournamentId: string) => ['tournaments', tournamentId, 'invitations'] as const,
}

export function previewInvitation(invitationId: string, token: string): Promise<InvitationPreview> {
  return requestDecoded(
    `/api/invitations/${invitationId}/preview`,
    decodeInvitationPreview,
    jsonRequest('POST', { token }),
  )
}

export function registerInvitation(
  invitationId: string,
  token: string,
  input: InvitationRegistrationInput,
): Promise<InvitationRegistrationResult> {
  return requestDecoded(
    `/api/invitations/${invitationId}/register`,
    decodeInvitationRegistration,
    jsonRequest('POST', { token, ...input }),
  )
}

export function acceptInvitation(
  invitationId: string,
  token: string,
  csrfToken: string,
): Promise<InvitationAcceptanceResult> {
  return requestDecoded(
    `/api/invitations/${invitationId}/accept`,
    decodeInvitationAcceptance,
    jsonRequest('POST', { token }, csrfToken),
  )
}

export function listInvitations(tournamentId: string): Promise<InvitationMetadata[]> {
  return requestDecoded(
    `/api/tournaments/${tournamentId}/invitations`,
    (value) => decodeExpectedInvitationList(value, tournamentId),
  )
}

export function issueInvitation(
  tournamentId: string,
  input: InvitationIssueInput,
  csrfToken: string,
): Promise<InvitationSecretResult> {
  return requestDecoded(
    `/api/tournaments/${tournamentId}/invitations`,
    (value) => decodeExpectedInvitationSecret(value, tournamentId),
    jsonRequest('POST', input, csrfToken),
  )
}

export function rotateInvitation(
  tournamentId: string,
  invitationId: string,
  csrfToken: string,
): Promise<InvitationSecretResult> {
  return requestDecoded(
    `/api/tournaments/${tournamentId}/invitations/${invitationId}/rotate`,
    (value) => decodeExpectedInvitationSecret(value, tournamentId, invitationId),
    jsonRequest('POST', {}, csrfToken),
  )
}

export function revokeInvitation(
  tournamentId: string,
  invitationId: string,
  csrfToken: string,
): Promise<void> {
  return requestNoContent(`/api/tournaments/${tournamentId}/invitations/${invitationId}`, {
    method: 'DELETE',
    headers: { 'x-csrf-token': csrfToken },
  })
}

export function buildInvitationUrl(origin: string, invitationId: string, token: string): string {
  const url = new URL(`/join/${invitationId}`, origin)
  url.hash = new URLSearchParams({ token }).toString()
  return url.toString()
}
