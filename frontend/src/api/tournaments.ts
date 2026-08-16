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
import { requestDecoded } from './http'
import type {
  Round,
  RoundStatus,
  ScoringFormat,
  ScoringMode,
  Tournament,
  TournamentStatus,
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
  root: ['tournaments'] as const,
  publicList: ['tournaments', 'public'] as const,
  mineRoot: ['tournaments', 'mine'] as const,
  mine: (userId: string) => ['tournaments', 'mine', userId] as const,
  detail: (tournamentId: string) => ['tournaments', tournamentId, 'detail'] as const,
  players: (tournamentId: string) => ['tournaments', tournamentId, 'players'] as const,
  rounds: (tournamentId: string) => ['tournaments', tournamentId, 'rounds'] as const,
}

export function withCreatedTournament(current: Tournament[] | undefined, created: Tournament): Tournament[] {
  return [created, ...(current ?? []).filter((tournament) => tournament.id !== created.id)]
}

export const tournamentApi = {
  mine: () => requestDecoded('/api/me/tournaments', decodeMyTournaments),
  detail: (id: string) => requestDecoded(`/api/tournaments/${id}`, (value) => decodeTournament(value)),
  rounds: (id: string) => requestDecoded(`/api/tournaments/${id}/rounds`, (value) =>
    decodeArray(value, 'rounds', decodeRound, 'rundedata')),
}
