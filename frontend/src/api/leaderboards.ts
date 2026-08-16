import type {
  CurrentTeam,
  LeaderboardMember,
  LeaderboardMetric,
  LeaderboardOwner,
  ParticipantStatus,
  RoundLeaderboard,
  RoundLeaderboardEntry,
  RoundStatus,
  ScoringFormat,
  TournamentLeaderboard,
  TournamentLeaderboardEntry,
} from './types'
import {
  decodeArray,
  decodeBoolean,
  decodeInteger,
  decodeObject,
  decodeString,
  decodeUuid,
  invalidData,
} from './decoder'
import { privateWorkspaceKeys } from './privateWorkspace'

export const leaderboardKeys = {
  round: (userId: string, roundId: string, metric: LeaderboardMetric) =>
    [...privateWorkspaceKeys.user(userId), 'leaderboards', 'round', roundId, metric] as const,
  tournament: (userId: string, tournamentId: string, metric: LeaderboardMetric) =>
    [...privateWorkspaceKeys.user(userId), 'leaderboards', 'tournament', tournamentId, metric] as const,
}

function invalid(path: string): never {
  return invalidData('resultatdata', path)
}

function nullableInteger(value: unknown, path: string): number | null {
  return value === null ? null : decodeInteger(value, path, 1, undefined, 'resultatdata')
}

function metric(value: unknown, path: string): LeaderboardMetric {
  if (value === 'gross' || value === 'net') return value
  return invalid(path)
}

function roundStatus(value: unknown, path: string): RoundStatus {
  if (value === 'draft' || value === 'open' || value === 'completed' || value === 'locked') return value
  return invalid(path)
}

function scoringFormat(value: unknown, path: string): ScoringFormat {
  if (value === 'individual_stroke_play' || value === 'team_scramble') return value
  return invalid(path)
}

function participantStatus(value: unknown, path: string): ParticipantStatus {
  if (value === 'active' || value === 'withdrawn') return value
  return invalid(path)
}

function owner(value: unknown, path: string): LeaderboardOwner {
  const data = decodeObject(value, path, 'resultatdata')
  if (data.type === 'player') return { type: 'player', id: decodeUuid(data.id, `${path}.id`, 'resultatdata') }
  if (data.type === 'team') return { type: 'team', id: decodeUuid(data.id, `${path}.id`, 'resultatdata') }
  return invalid(`${path}.type`)
}

function member(value: unknown, path: string): LeaderboardMember {
  const data = decodeObject(value, path, 'resultatdata')
  return {
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'resultatdata'),
    display_name: decodeString(data.display_name, `${path}.display_name`, 'resultatdata'),
    display_order: data.display_order === null ? null : decodeInteger(data.display_order, `${path}.display_order`, undefined, undefined, 'resultatdata'),
  }
}

function roundEntry(value: unknown, path: string): RoundLeaderboardEntry {
  const data = decodeObject(value, path, 'resultatdata')
  return {
    position: nullableInteger(data.position, `${path}.position`),
    tied: decodeBoolean(data.tied, `${path}.tied`, 'resultatdata'),
    owner: owner(data.owner, `${path}.owner`),
    owner_name: decodeString(data.owner_name, `${path}.owner_name`, 'resultatdata'),
    members: decodeArray(data.members, `${path}.members`, member, 'resultatdata'),
    holes_scored: decodeInteger(data.holes_scored, `${path}.holes_scored`, 0, undefined, 'resultatdata'),
    number_of_holes: decodeInteger(data.number_of_holes, `${path}.number_of_holes`, 1, undefined, 'resultatdata'),
    complete: decodeBoolean(data.complete, `${path}.complete`, 'resultatdata'),
    confirmed: decodeBoolean(data.confirmed, `${path}.confirmed`, 'resultatdata'),
    playing_handicap: decodeInteger(data.playing_handicap, `${path}.playing_handicap`, undefined, undefined, 'resultatdata'),
    gross_total: decodeInteger(data.gross_total, `${path}.gross_total`, undefined, undefined, 'resultatdata'),
    net_total: decodeInteger(data.net_total, `${path}.net_total`, undefined, undefined, 'resultatdata'),
    par_played: decodeInteger(data.par_played, `${path}.par_played`, 0, undefined, 'resultatdata'),
    score_to_par: decodeInteger(data.score_to_par, `${path}.score_to_par`, undefined, undefined, 'resultatdata'),
  }
}

function currentTeam(value: unknown, path: string): CurrentTeam | null {
  if (value === null) return null
  const data = decodeObject(value, path, 'resultatdata')
  return {
    round_id: decodeUuid(data.round_id, `${path}.round_id`, 'resultatdata'),
    team_id: decodeUuid(data.team_id, `${path}.team_id`, 'resultatdata'),
    team_name: decodeString(data.team_name, `${path}.team_name`, 'resultatdata'),
  }
}

function tournamentEntry(value: unknown, path: string): TournamentLeaderboardEntry {
  const data = decodeObject(value, path, 'resultatdata')
  return {
    position: nullableInteger(data.position, `${path}.position`),
    tied: decodeBoolean(data.tied, `${path}.tied`, 'resultatdata'),
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'resultatdata'),
    display_name: decodeString(data.display_name, `${path}.display_name`, 'resultatdata'),
    status: participantStatus(data.status, `${path}.status`),
    completed_rounds: decodeInteger(data.completed_rounds, `${path}.completed_rounds`, 0, undefined, 'resultatdata'),
    gross_total: decodeInteger(data.gross_total, `${path}.gross_total`, undefined, undefined, 'resultatdata'),
    net_total: decodeInteger(data.net_total, `${path}.net_total`, undefined, undefined, 'resultatdata'),
    current_team: currentTeam(data.current_team, `${path}.current_team`),
  }
}

export function decodeRoundLeaderboard(
  value: unknown,
  expectedRoundId: string,
  expectedMetric: LeaderboardMetric,
): RoundLeaderboard {
  const data = decodeObject(value, 'leaderboard', 'resultatdata')
  const decoded: RoundLeaderboard = {
    round_id: decodeUuid(data.round_id, 'leaderboard.round_id', 'resultatdata'),
    tournament_id: decodeUuid(data.tournament_id, 'leaderboard.tournament_id', 'resultatdata'),
    status: roundStatus(data.status, 'leaderboard.status'),
    scoring_format: scoringFormat(data.scoring_format, 'leaderboard.scoring_format'),
    metric: metric(data.metric, 'leaderboard.metric'),
    number_of_holes: decodeInteger(data.number_of_holes, 'leaderboard.number_of_holes', 1, undefined, 'resultatdata'),
    entries: decodeArray(data.entries, 'leaderboard.entries', roundEntry, 'resultatdata'),
  }
  if (decoded.round_id !== expectedRoundId || decoded.metric !== expectedMetric) invalid('leaderboard.identity')
  return decoded
}

export function decodeTournamentLeaderboard(
  value: unknown,
  expectedTournamentId: string,
  expectedMetric: LeaderboardMetric,
): TournamentLeaderboard {
  const data = decodeObject(value, 'leaderboard', 'resultatdata')
  const decoded: TournamentLeaderboard = {
    tournament_id: decodeUuid(data.tournament_id, 'leaderboard.tournament_id', 'resultatdata'),
    metric: metric(data.metric, 'leaderboard.metric'),
    current_round_id: data.current_round_id === null ? null : decodeUuid(data.current_round_id, 'leaderboard.current_round_id', 'resultatdata'),
    included_round_ids: decodeArray(data.included_round_ids, 'leaderboard.included_round_ids', (item, path) => decodeUuid(item, path, 'resultatdata'), 'resultatdata'),
    entries: decodeArray(data.entries, 'leaderboard.entries', tournamentEntry, 'resultatdata'),
  }
  if (decoded.tournament_id !== expectedTournamentId || decoded.metric !== expectedMetric) invalid('leaderboard.identity')
  return decoded
}
