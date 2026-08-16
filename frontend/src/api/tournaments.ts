import {
  decodeArray,
  decodeBoolean,
  decodeDate,
  decodeInteger,
  decodeNumber,
  decodeObject,
  decodeString,
  decodeTimestamp,
  decodeUuid,
  invalidData,
} from './decoder'
import { requestDecoded } from './http'
import { jsonRequest } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'
import type {
  Round,
  RoundStatus,
  ScoringFormat,
  ScoringMode,
  Tournament,
  TournamentStatus,
  TournamentHandicapCorrection,
  TournamentHandicapCorrectionState,
  TournamentPlayer,
  TournamentPlayerRoster,
} from './types'

export type TournamentRole = 'admin' | 'scorer' | 'player' | 'viewer'

export interface MyTournament {
  tournament: Tournament
  role: TournamentRole
  player_id: string | null
}

function tournamentStatus(value: unknown, path: string): TournamentStatus {
  if (value === 'draft' || value === 'active' || value === 'completed' || value === 'archived') return value
  return invalidData('turneringsdata', path)
}

function scoringMode(value: unknown, path: string): ScoringMode {
  if (value === 'individual' || value === 'team' || value === 'combined') return value
  return invalidData('turneringsdata', path)
}

function roundStatus(value: unknown, path: string): RoundStatus {
  if (value === 'draft' || value === 'open' || value === 'completed' || value === 'locked') return value
  return invalidData('rundedata', path)
}

function scoringFormat(value: unknown, path: string): ScoringFormat {
  if (value === 'individual_stroke_play' || value === 'team_scramble') return value
  return invalidData('rundedata', path)
}

function tournamentRole(value: unknown, path: string): TournamentRole {
  if (value === 'admin' || value === 'scorer' || value === 'player' || value === 'viewer') return value
  return invalidData('turneringsmedlemskap', path)
}

function participantStatus(value: unknown, path: string): TournamentPlayer['status'] {
  if (value === 'active' || value === 'withdrawn') return value
  return invalidData('turneringsspillerdata', path)
}

export function decodeTournamentPlayer(value: unknown, path = 'player'): TournamentPlayer {
  const data = decodeObject(value, path, 'turneringsspillerdata')
  return {
    tournament_id: decodeUuid(data.tournament_id, `${path}.tournament_id`, 'turneringsspillerdata'),
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'turneringsspillerdata'),
    display_name: decodeString(data.display_name, `${path}.display_name`, 'turneringsspillerdata'),
    tournament_handicap: decodeNumber(data.tournament_handicap, `${path}.tournament_handicap`, -10, 54, 'turneringsspillerdata'),
    seed: data.seed === null ? null : decodeInteger(data.seed, `${path}.seed`, undefined, undefined, 'turneringsspillerdata'),
    status: participantStatus(data.status, `${path}.status`),
    created_at: decodeTimestamp(data.created_at, `${path}.created_at`, 'turneringsspillerdata'),
    updated_at: decodeTimestamp(data.updated_at, `${path}.updated_at`, 'turneringsspillerdata'),
  }
}

function decodeHandicapCorrectionState(value: unknown): TournamentHandicapCorrectionState {
  const data = decodeObject(value, 'roster.handicap_correction', 'turneringsspillerdata')
  if (data.state === 'editable') return { state: 'editable' }
  if (data.state === 'locked' && (data.reason === 'round_opened' || data.reason === 'snapshot_captured')) {
    return { state: 'locked', reason: data.reason }
  }
  return invalidData('turneringsspillerdata', 'roster.handicap_correction')
}

export function decodeTournamentPlayerRoster(value: unknown): TournamentPlayerRoster {
  const data = decodeObject(value, 'roster', 'turneringsspillerdata')
  return {
    handicap_correction: decodeHandicapCorrectionState(data.handicap_correction),
    players: decodeArray(data.players, 'roster.players', decodeTournamentPlayer, 'turneringsspillerdata'),
  }
}

export function decodeTournamentHandicapCorrection(value: unknown): TournamentHandicapCorrection {
  const data = decodeObject(value, 'correction', 'turneringsspillerdata')
  const audit = decodeObject(data.audit, 'correction.audit', 'turneringsspillerdata')
  return {
    player: decodeTournamentPlayer(data.player, 'correction.player'),
    audit: {
      id: decodeUuid(audit.id, 'correction.audit.id', 'turneringsspillerdata'),
      tournament_id: decodeUuid(audit.tournament_id, 'correction.audit.tournament_id', 'turneringsspillerdata'),
      player_id: decodeUuid(audit.player_id, 'correction.audit.player_id', 'turneringsspillerdata'),
      handicap_index: decodeNumber(audit.handicap_index, 'correction.audit.handicap_index', -10, 54, 'turneringsspillerdata'),
      effective_from: decodeTimestamp(audit.effective_from, 'correction.audit.effective_from', 'turneringsspillerdata'),
      changed_by: audit.changed_by === null ? null : decodeUuid(audit.changed_by, 'correction.audit.changed_by', 'turneringsspillerdata'),
      reason: audit.reason === null ? null : decodeString(audit.reason, 'correction.audit.reason', 'turneringsspillerdata'),
      created_at: decodeTimestamp(audit.created_at, 'correction.audit.created_at', 'turneringsspillerdata'),
    },
  }
}

export function decodeTournament(value: unknown, path = 'tournament'): Tournament {
  const data = decodeObject(value, path, 'turneringsdata')
  return {
    id: decodeUuid(data.id, `${path}.id`, 'turneringsdata'),
    name: decodeString(data.name, `${path}.name`, 'turneringsdata'),
    description: decodeString(data.description, `${path}.description`, 'turneringsdata'),
    start_date: decodeDate(data.start_date, `${path}.start_date`, 'turneringsdata'),
    end_date: decodeDate(data.end_date, `${path}.end_date`, 'turneringsdata'),
    number_of_rounds: decodeInteger(data.number_of_rounds, `${path}.number_of_rounds`, 1, 30, 'turneringsdata'),
    status: tournamentStatus(data.status, `${path}.status`),
    scoring_mode: scoringMode(data.scoring_mode, `${path}.scoring_mode`),
    created_at: decodeTimestamp(data.created_at, `${path}.created_at`, 'turneringsdata'),
    updated_at: decodeTimestamp(data.updated_at, `${path}.updated_at`, 'turneringsdata'),
  }
}

export function decodeRound(value: unknown, path = 'round'): Round {
  const data = decodeObject(value, path, 'rundedata')
  return {
    id: decodeUuid(data.id, `${path}.id`, 'rundedata'),
    tournament_id: decodeUuid(data.tournament_id, `${path}.tournament_id`, 'rundedata'),
    round_number: decodeInteger(data.round_number, `${path}.round_number`, 1, 30, 'rundedata'),
    name: decodeString(data.name, `${path}.name`, 'rundedata'),
    round_date: decodeDate(data.round_date, `${path}.round_date`, 'rundedata'),
    course_id: data.course_id === null ? null : decodeUuid(data.course_id, `${path}.course_id`, 'rundedata'),
    course_name: decodeString(data.course_name, `${path}.course_name`, 'rundedata'),
    tee_id: data.tee_id === null ? null : decodeUuid(data.tee_id, `${path}.tee_id`, 'rundedata'),
    tee_name: decodeString(data.tee_name, `${path}.tee_name`, 'rundedata'),
    number_of_holes: decodeInteger(data.number_of_holes, `${path}.number_of_holes`, 1, 36, 'rundedata'),
    status: roundStatus(data.status, `${path}.status`),
    handicap_enabled: decodeBoolean(data.handicap_enabled, `${path}.handicap_enabled`, 'rundedata'),
    handicap_allowance_percent: decodeInteger(data.handicap_allowance_percent, `${path}.handicap_allowance_percent`, 0, 100, 'rundedata'),
    scoring_format: scoringFormat(data.scoring_format, `${path}.scoring_format`),
    created_at: decodeTimestamp(data.created_at, `${path}.created_at`, 'rundedata'),
    updated_at: decodeTimestamp(data.updated_at, `${path}.updated_at`, 'rundedata'),
  }
}

export function decodeMyTournaments(value: unknown): MyTournament[] {
  return decodeArray(value, 'tournaments', (item, path) => {
    const data = decodeObject(item, path, 'turneringsmedlemskap')
    return {
      tournament: decodeTournament(data.tournament, `${path}.tournament`),
      role: tournamentRole(data.role, `${path}.role`),
      player_id: data.player_id === null
        ? null
        : decodeUuid(data.player_id, `${path}.player_id`, 'turneringsmedlemskap'),
    }
  }, 'turneringsmedlemskap')
}

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
  detail: (id: string) => requestDecoded(`/api/tournaments/${id}`, (value) => decodeTournament(value)),
  rounds: (id: string) => requestDecoded(`/api/tournaments/${id}/rounds`, (value) =>
    decodeArray(value, 'rounds', decodeRound, 'rundedata')),
  players: (id: string) => requestDecoded(`/api/tournaments/${id}/players`, decodeTournamentPlayerRoster),
  correctHandicap: (
    tournamentId: string,
    playerId: string,
    input: { handicap_index: number; reason: string },
    csrfToken: string,
  ) => requestDecoded(
    `/api/tournaments/${tournamentId}/players/${playerId}/handicap-corrections`,
    decodeTournamentHandicapCorrection,
    jsonRequest('POST', input, csrfToken),
  ),
}
