import { requestDecoded } from './http'
import { jsonRequest } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'
import {
  decodeExpectedTournament,
  decodeMyTournaments,
  decodeTournamentHandicapCorrection,
  decodeTournamentPlayerRoster,
  decodeTournamentRounds,
} from './tournaments/decoders'
import type { Tournament } from './types'

export * from './tournaments/decoders'

export const tournamentKeys = {
  root: (userId: string) => [...privateWorkspaceKeys.user(userId), 'tournaments'] as const,
  list: (userId: string) => [...privateWorkspaceKeys.user(userId), 'tournaments', 'list'] as const,
  mine: (userId: string) => [...privateWorkspaceKeys.user(userId), 'tournaments', 'mine'] as const,
  detail: (userId: string, tournamentId: string) =>
    [...privateWorkspaceKeys.user(userId), 'tournaments', tournamentId, 'detail'] as const,
  players: (userId: string, tournamentId: string) =>
    [...privateWorkspaceKeys.user(userId), 'tournaments', tournamentId, 'players'] as const,
  rounds: (userId: string, tournamentId: string) =>
    [...privateWorkspaceKeys.user(userId), 'tournaments', tournamentId, 'rounds'] as const,
  round: (userId: string, roundId: string) =>
    [...privateWorkspaceKeys.user(userId), 'rounds', roundId, 'detail'] as const,
  teams: (userId: string, roundId: string) =>
    [...privateWorkspaceKeys.user(userId), 'rounds', roundId, 'teams'] as const,
}

export function withCreatedTournament(current: Tournament[] | undefined, created: Tournament): Tournament[] {
  return [created, ...(current ?? []).filter((tournament) => tournament.id !== created.id)]
}

export const tournamentApi = {
  mine: () => requestDecoded('/api/me/tournaments', decodeMyTournaments),
  detail: (id: string) => requestDecoded(`/api/tournaments/${id}`, (value) => decodeExpectedTournament(value, id)),
  rounds: (id: string) => requestDecoded(`/api/tournaments/${id}/rounds`, (value) => decodeTournamentRounds(value, id)),
  players: (id: string) => requestDecoded(`/api/tournaments/${id}/players`, (value) => decodeTournamentPlayerRoster(value, id)),
  updateCountedRounds: (
    tournamentId: string,
    input: { counted_rounds: number; expected_tournament_updated_at: string },
    csrfToken: string,
  ) => requestDecoded(
    `/api/tournaments/${tournamentId}/counted-rounds`,
    (value) => decodeExpectedTournament(value, tournamentId),
    {
      method: 'PATCH',
      headers: { 'content-type': 'application/json', 'x-csrf-token': csrfToken },
      body: JSON.stringify(input),
    },
  ),
  start: (
    tournamentId: string,
    input: { expected_tournament_updated_at: string },
    csrfToken: string,
  ) => requestDecoded(
    `/api/tournaments/${tournamentId}/start`,
    (value) => decodeExpectedTournament(value, tournamentId),
    jsonRequest('POST', input, csrfToken),
  ),
  correctHandicap: (
    tournamentId: string,
    playerId: string,
    input: { handicap_index: number; reason: string },
    csrfToken: string,
  ) => requestDecoded(
    `/api/tournaments/${tournamentId}/players/${playerId}/handicap-corrections`,
    (value) => decodeTournamentHandicapCorrection(value, tournamentId, playerId),
    jsonRequest('POST', input, csrfToken),
  ),
}
