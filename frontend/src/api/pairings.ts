import {
  decodeArray,
  decodeBoolean,
  decodeInteger,
  decodeObject,
  decodeString,
  decodeTimestamp,
  decodeUuid,
  invalidData,
} from './decoder'
import { jsonRequest, requestDecoded } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'
import { SCORING_FORMATS } from './scoringFormats'
import type { ParticipantStatus, RoundStatus, ScoringFormat } from './types'

export interface PairingEntrant {
  player_id: string
  display_name: string
  status: ParticipantStatus
  player_active: boolean
}

export interface PairingMember {
  player_id: string
  display_name: string
  display_order: number | null
}

export interface PairingGroup {
  id: string
  name: string
  starting_hole: number | null
  tee_time: string | null
  created_at: string
  updated_at: string
  members: PairingMember[]
}

export interface RoundPairings {
  round_id: string
  tournament_id: string
  status: RoundStatus
  scoring_format: ScoringFormat
  updated_at: string
  active_entrants: PairingEntrant[]
  inactive_entrants: PairingEntrant[]
  teams: PairingGroup[]
  flights: PairingGroup[]
  legacy_individual_groups: PairingGroup[]
}

export interface PairingMemberInput { player_id: string }
export interface PairingTeamInput {
  id: string
  name: string
  members: PairingMemberInput[]
  schedule_flight_id: string | null
}
export interface PairingFlightInput {
  id: string
  name: string
  starting_hole: number | null
  tee_time: string | null
  members: PairingMemberInput[]
}
export interface LegacyConversionInput { team_id: string; flight_id: string }
export interface PairingReplacement {
  expected_round_updated_at: string
  teams: PairingTeamInput[]
  flights: PairingFlightInput[]
  legacy_conversions: LegacyConversionInput[]
}

const statuses: readonly RoundStatus[] = ['draft', 'open', 'completed', 'locked']
const formats: readonly ScoringFormat[] = SCORING_FORMATS
const participantStatuses: readonly ParticipantStatus[] = ['active', 'withdrawn']
const timePattern = /^([01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?$/

function decodeEnum<T extends string>(value: unknown, allowed: readonly T[], path: string): T {
  const decoded = decodeString(value, path, 'spillegruppedata')
  if (!allowed.some((item) => item === decoded)) invalidData('spillegruppedata', path)
  return decoded as T
}

function decodeNullable<T>(value: unknown, decode: (item: unknown) => T): T | null {
  return value === null ? null : decode(value)
}

function decodeEntrant(value: unknown, path: string): PairingEntrant {
  const data = decodeObject(value, path, 'spillegruppedata')
  return {
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'spillegruppedata'),
    display_name: decodeString(data.display_name, `${path}.display_name`, 'spillegruppedata'),
    status: decodeEnum(data.status, participantStatuses, `${path}.status`),
    player_active: decodeBoolean(data.player_active, `${path}.player_active`, 'spillegruppedata'),
  }
}

function decodeMember(value: unknown, path: string): PairingMember {
  const data = decodeObject(value, path, 'spillegruppedata')
  return {
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'spillegruppedata'),
    display_name: decodeString(data.display_name, `${path}.display_name`, 'spillegruppedata'),
    display_order: decodeNullable(data.display_order, (item) =>
      decodeInteger(item, `${path}.display_order`, 0, 32_767, 'spillegruppedata')),
  }
}

function decodeTime(value: unknown, path: string): string {
  const decoded = decodeString(value, path, 'spillegruppedata')
  if (!timePattern.test(decoded)) invalidData('spillegruppedata', path)
  return decoded
}

function decodeGroup(value: unknown, path: string): PairingGroup {
  const data = decodeObject(value, path, 'spillegruppedata')
  return {
    id: decodeUuid(data.id, `${path}.id`, 'spillegruppedata'),
    name: decodeString(data.name, `${path}.name`, 'spillegruppedata'),
    starting_hole: decodeNullable(data.starting_hole, (item) =>
      decodeInteger(item, `${path}.starting_hole`, 1, 36, 'spillegruppedata')),
    tee_time: decodeNullable(data.tee_time, (item) => decodeTime(item, `${path}.tee_time`)),
    created_at: decodeTimestamp(data.created_at, `${path}.created_at`, 'spillegruppedata'),
    updated_at: decodeTimestamp(data.updated_at, `${path}.updated_at`, 'spillegruppedata'),
    members: decodeArray(data.members, `${path}.members`, decodeMember, 'spillegruppedata'),
  }
}

export function decodeRoundPairings(value: unknown): RoundPairings {
  const data = decodeObject(value, 'pairings', 'spillegruppedata')
  return {
    round_id: decodeUuid(data.round_id, 'pairings.round_id', 'spillegruppedata'),
    tournament_id: decodeUuid(data.tournament_id, 'pairings.tournament_id', 'spillegruppedata'),
    status: decodeEnum(data.status, statuses, 'pairings.status'),
    scoring_format: decodeEnum(data.scoring_format, formats, 'pairings.scoring_format'),
    updated_at: decodeTimestamp(data.updated_at, 'pairings.updated_at', 'spillegruppedata'),
    active_entrants: decodeArray(data.active_entrants, 'pairings.active_entrants', decodeEntrant, 'spillegruppedata'),
    inactive_entrants: decodeArray(data.inactive_entrants, 'pairings.inactive_entrants', decodeEntrant, 'spillegruppedata'),
    teams: decodeArray(data.teams, 'pairings.teams', decodeGroup, 'spillegruppedata'),
    flights: decodeArray(data.flights, 'pairings.flights', decodeGroup, 'spillegruppedata'),
    legacy_individual_groups: decodeArray(data.legacy_individual_groups, 'pairings.legacy_individual_groups', decodeGroup, 'spillegruppedata'),
  }
}

function decodeExpectedRoundPairings(value: unknown, roundId: string): RoundPairings {
  const decoded = decodeRoundPairings(value)
  if (decoded.round_id !== roundId) invalidData('spillegruppedata', 'pairings.round_id identity')
  return decoded
}

export const pairingKeys = {
  detail: (userId: string, roundId: string) =>
    [...privateWorkspaceKeys.user(userId), 'rounds', roundId, 'pairings'] as const,
}

export const pairingApi = {
  get: (roundId: string) => requestDecoded(
    `/api/rounds/${roundId}/pairings`,
    (value) => decodeExpectedRoundPairings(value, roundId),
  ),
  replace: (roundId: string, replacement: PairingReplacement, csrfToken: string) =>
    requestDecoded(
      `/api/rounds/${roundId}/pairings`,
      (value) => decodeExpectedRoundPairings(value, roundId),
      jsonRequest('PUT', replacement, csrfToken),
    ),
}
