import {
  decodeArray,
  decodeBoolean,
  decodeInteger,
  decodeNumber,
  decodeObject,
  decodeString,
  decodeTimestamp,
  decodeUuid,
  invalidData,
} from './decoder'
import { jsonRequest, requestDecoded } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'
import { decodeRound } from './tournaments'
import type { Round } from './types'

export type ReadinessIssueCode =
  | 'round_not_draft'
  | 'tournament_not_openable'
  | 'no_active_entrants'
  | 'missing_team_assignment'
  | 'ineligible_team_assignment'
  | 'empty_team'
  | 'invalid_scramble_team_size'
  | 'invalid_foursomes_team_size'
  | 'missing_flight_assignment'
  | 'ineligible_flight_assignment'
  | 'empty_flight'
  | 'legacy_individual_groups_present'
  | 'team_split_across_flights'
  | 'missing_course'
  | 'missing_tee'
  | 'mismatched_course_tee'
  | 'missing_handicap_ratings'
  | 'invalid_hole_count'
  | 'invalid_hole_numbers'
  | 'invalid_stroke_indexes'

const readinessIssueCodes = [
  'round_not_draft', 'tournament_not_openable', 'no_active_entrants',
  'missing_team_assignment', 'ineligible_team_assignment', 'empty_team',
  'invalid_scramble_team_size', 'invalid_foursomes_team_size',
  'missing_flight_assignment', 'ineligible_flight_assignment', 'empty_flight',
  'legacy_individual_groups_present', 'team_split_across_flights', 'missing_course',
  'missing_tee', 'mismatched_course_tee', 'missing_handicap_ratings',
  'invalid_hole_count', 'invalid_hole_numbers', 'invalid_stroke_indexes',
] as const satisfies readonly ReadinessIssueCode[]

export interface PairingValidation {
  round_id: string
  ready: boolean
  issues: Array<{ code: ReadinessIssueCode; message: string }>
  missing_players: Array<{ player_id: string; display_name: string }>
  ineligible_players: Array<{ player_id: string; display_name: string }>
  team_sizes: Array<{ team_id: string; team_name: string; player_count: number }>
  missing_flight_players: Array<{ player_id: string; display_name: string }>
  ineligible_flight_players: Array<{ player_id: string; display_name: string }>
  flight_sizes: Array<{ flight_id: string; flight_name: string; player_count: number }>
  legacy_individual_groups: Array<{ team_id: string; team_name: string; player_count: number }>
  split_teams: Array<{ team_id: string; team_name: string; player_count: number }>
}

export interface RoundHandicapSnapshot {
  round_id: string
  tournament_id: string
  player_id: string
  handicap_index: number
  course_handicap: number
  playing_handicap: number
  captured_at: string
}

export interface RoundTeamHandicapSnapshot {
  round_id: string
  tournament_id: string
  team_id: string
  playing_handicap: number
  captured_at: string
}

export interface OpenRoundResult {
  round: Round
  handicap_snapshots: RoundHandicapSnapshot[]
  team_handicap_snapshots: RoundTeamHandicapSnapshot[]
}

function invalid(path: string): never {
  return invalidData('rundelivsløp', path)
}

function isReadinessIssueCode(value: string): value is ReadinessIssueCode {
  return readinessIssueCodes.some((allowed) => allowed === value)
}

function readinessIssue(value: unknown, path: string): PairingValidation['issues'][number] {
  const data = decodeObject(value, path, 'rundelivsløp')
  const code = decodeString(data.code, `${path}.code`, 'rundelivsløp')
  if (!isReadinessIssueCode(code)) invalid(`${path}.code`)
  return { code, message: decodeString(data.message, `${path}.message`, 'rundelivsløp') }
}

function readinessPlayer(value: unknown, path: string): PairingValidation['missing_players'][number] {
  const data = decodeObject(value, path, 'rundelivsløp')
  return {
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'rundelivsløp'),
    display_name: decodeString(data.display_name, `${path}.display_name`, 'rundelivsløp'),
  }
}

function teamSize(value: unknown, path: string): PairingValidation['team_sizes'][number] {
  const data = decodeObject(value, path, 'rundelivsløp')
  return {
    team_id: decodeUuid(data.team_id, `${path}.team_id`, 'rundelivsløp'),
    team_name: decodeString(data.team_name, `${path}.team_name`, 'rundelivsløp'),
    player_count: decodeInteger(data.player_count, `${path}.player_count`, 0, undefined, 'rundelivsløp'),
  }
}

function flightSize(value: unknown, path: string): PairingValidation['flight_sizes'][number] {
  const data = decodeObject(value, path, 'rundelivsløp')
  return {
    flight_id: decodeUuid(data.flight_id, `${path}.flight_id`, 'rundelivsløp'),
    flight_name: decodeString(data.flight_name, `${path}.flight_name`, 'rundelivsløp'),
    player_count: decodeInteger(data.player_count, `${path}.player_count`, 0, undefined, 'rundelivsløp'),
  }
}

export function decodePairingValidation(value: unknown, expectedRoundId: string): PairingValidation {
  const data = decodeObject(value, 'validation', 'rundelivsløp')
  const decoded: PairingValidation = {
    round_id: decodeUuid(data.round_id, 'validation.round_id', 'rundelivsløp'),
    ready: decodeBoolean(data.ready, 'validation.ready', 'rundelivsløp'),
    issues: decodeArray(data.issues, 'validation.issues', readinessIssue, 'rundelivsløp'),
    missing_players: decodeArray(data.missing_players, 'validation.missing_players', readinessPlayer, 'rundelivsløp'),
    ineligible_players: decodeArray(data.ineligible_players, 'validation.ineligible_players', readinessPlayer, 'rundelivsløp'),
    team_sizes: decodeArray(data.team_sizes, 'validation.team_sizes', teamSize, 'rundelivsløp'),
    missing_flight_players: decodeArray(data.missing_flight_players, 'validation.missing_flight_players', readinessPlayer, 'rundelivsløp'),
    ineligible_flight_players: decodeArray(data.ineligible_flight_players, 'validation.ineligible_flight_players', readinessPlayer, 'rundelivsløp'),
    flight_sizes: decodeArray(data.flight_sizes, 'validation.flight_sizes', flightSize, 'rundelivsløp'),
    legacy_individual_groups: decodeArray(data.legacy_individual_groups, 'validation.legacy_individual_groups', teamSize, 'rundelivsløp'),
    split_teams: decodeArray(data.split_teams, 'validation.split_teams', teamSize, 'rundelivsløp'),
  }
  if (decoded.round_id !== expectedRoundId || decoded.ready !== (decoded.issues.length === 0)) {
    invalid('validation.identity')
  }
  return decoded
}

function handicapSnapshot(value: unknown, path: string): RoundHandicapSnapshot {
  const data = decodeObject(value, path, 'rundelivsløp')
  return {
    round_id: decodeUuid(data.round_id, `${path}.round_id`, 'rundelivsløp'),
    tournament_id: decodeUuid(data.tournament_id, `${path}.tournament_id`, 'rundelivsløp'),
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'rundelivsløp'),
    handicap_index: decodeNumber(data.handicap_index, `${path}.handicap_index`, -10, 54, 'rundelivsløp'),
    course_handicap: decodeInteger(data.course_handicap, `${path}.course_handicap`, undefined, undefined, 'rundelivsløp'),
    playing_handicap: decodeInteger(data.playing_handicap, `${path}.playing_handicap`, undefined, undefined, 'rundelivsløp'),
    captured_at: decodeTimestamp(data.captured_at, `${path}.captured_at`, 'rundelivsløp'),
  }
}

function teamHandicapSnapshot(value: unknown, path: string): RoundTeamHandicapSnapshot {
  const data = decodeObject(value, path, 'rundelivsløp')
  return {
    round_id: decodeUuid(data.round_id, `${path}.round_id`, 'rundelivsløp'),
    tournament_id: decodeUuid(data.tournament_id, `${path}.tournament_id`, 'rundelivsløp'),
    team_id: decodeUuid(data.team_id, `${path}.team_id`, 'rundelivsløp'),
    playing_handicap: decodeInteger(data.playing_handicap, `${path}.playing_handicap`, undefined, undefined, 'rundelivsløp'),
    captured_at: decodeTimestamp(data.captured_at, `${path}.captured_at`, 'rundelivsløp'),
  }
}

export function decodeOpenRoundResult(value: unknown, expectedRoundId: string): OpenRoundResult {
  const data = decodeObject(value, 'opening', 'rundelivsløp')
  const round = decodeRound(data.round, 'opening.round')
  const handicapSnapshots = decodeArray(data.handicap_snapshots, 'opening.handicap_snapshots', handicapSnapshot, 'rundelivsløp')
  const teamSnapshots = decodeArray(data.team_handicap_snapshots, 'opening.team_handicap_snapshots', teamHandicapSnapshot, 'rundelivsløp')
  const validSnapshot = (snapshot: { round_id: string; tournament_id: string }) =>
    snapshot.round_id === round.id && snapshot.tournament_id === round.tournament_id
  const playerIds = new Set(handicapSnapshots.map((snapshot) => snapshot.player_id))
  const teamIds = new Set(teamSnapshots.map((snapshot) => snapshot.team_id))
  if (round.id !== expectedRoundId || round.status !== 'open'
    || handicapSnapshots.some((snapshot) => !validSnapshot(snapshot))
    || teamSnapshots.some((snapshot) => !validSnapshot(snapshot))
    || playerIds.size !== handicapSnapshots.length || teamIds.size !== teamSnapshots.length) {
    invalid('opening.identity')
  }
  return { round, handicap_snapshots: handicapSnapshots, team_handicap_snapshots: teamSnapshots }
}

export const roundLifecycleKeys = {
  validation: (userId: string, roundId: string) =>
    [...privateWorkspaceKeys.user(userId), 'rounds', roundId, 'pairing-validation'] as const,
}

export const roundLifecycleApi = {
  validation: (roundId: string) => requestDecoded(`/api/rounds/${roundId}/pairing-validation`,
    (value) => decodePairingValidation(value, roundId)),
  open: (roundId: string, csrfToken: string) => requestDecoded(`/api/rounds/${roundId}/open`,
    (value) => decodeOpenRoundResult(value, roundId), jsonRequest('POST', {}, csrfToken)),
}
