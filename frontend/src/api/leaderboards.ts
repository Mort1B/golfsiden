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

export const leaderboardKeys = {
  round: (roundId: string, metric: LeaderboardMetric) =>
    ['leaderboards', 'round', roundId, metric] as const,
  tournament: (tournamentId: string, metric: LeaderboardMetric) =>
    ['leaderboards', 'tournament', tournamentId, metric] as const,
}

const uuidPattern = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i

function invalid(path: string): never {
  throw new Error(`Ugyldig resultatdata fra serveren (${path})`)
}

function object(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) invalid(path)
  return value as Record<string, unknown>
}

function string(value: unknown, path: string): string {
  if (typeof value !== 'string') invalid(path)
  return value
}

function uuid(value: unknown, path: string): string {
  const decoded = string(value, path)
  if (!uuidPattern.test(decoded)) invalid(path)
  return decoded
}

function integer(value: unknown, path: string, minimum?: number): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value)) invalid(path)
  if (minimum !== undefined && value < minimum) invalid(path)
  return value
}

function boolean(value: unknown, path: string): boolean {
  if (typeof value !== 'boolean') invalid(path)
  return value
}

function nullableInteger(value: unknown, path: string): number | null {
  return value === null ? null : integer(value, path, 1)
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

function array<T>(value: unknown, path: string, decode: (item: unknown, path: string) => T): T[] {
  if (!Array.isArray(value)) invalid(path)
  return value.map((item, index) => decode(item, `${path}[${index}]`))
}

function owner(value: unknown, path: string): LeaderboardOwner {
  const data = object(value, path)
  if (data.type === 'player') return { type: 'player', id: uuid(data.id, `${path}.id`) }
  if (data.type === 'team') return { type: 'team', id: uuid(data.id, `${path}.id`) }
  return invalid(`${path}.type`)
}

function member(value: unknown, path: string): LeaderboardMember {
  const data = object(value, path)
  return {
    player_id: uuid(data.player_id, `${path}.player_id`),
    display_name: string(data.display_name, `${path}.display_name`),
    display_order: data.display_order === null ? null : integer(data.display_order, `${path}.display_order`),
  }
}

function roundEntry(value: unknown, path: string): RoundLeaderboardEntry {
  const data = object(value, path)
  return {
    position: nullableInteger(data.position, `${path}.position`),
    tied: boolean(data.tied, `${path}.tied`),
    owner: owner(data.owner, `${path}.owner`),
    owner_name: string(data.owner_name, `${path}.owner_name`),
    members: array(data.members, `${path}.members`, member),
    holes_scored: integer(data.holes_scored, `${path}.holes_scored`, 0),
    number_of_holes: integer(data.number_of_holes, `${path}.number_of_holes`, 1),
    complete: boolean(data.complete, `${path}.complete`),
    confirmed: boolean(data.confirmed, `${path}.confirmed`),
    playing_handicap: integer(data.playing_handicap, `${path}.playing_handicap`),
    gross_total: integer(data.gross_total, `${path}.gross_total`),
    net_total: integer(data.net_total, `${path}.net_total`),
    par_played: integer(data.par_played, `${path}.par_played`, 0),
    score_to_par: integer(data.score_to_par, `${path}.score_to_par`),
  }
}

function currentTeam(value: unknown, path: string): CurrentTeam | null {
  if (value === null) return null
  const data = object(value, path)
  return {
    round_id: uuid(data.round_id, `${path}.round_id`),
    team_id: uuid(data.team_id, `${path}.team_id`),
    team_name: string(data.team_name, `${path}.team_name`),
  }
}

function tournamentEntry(value: unknown, path: string): TournamentLeaderboardEntry {
  const data = object(value, path)
  return {
    position: nullableInteger(data.position, `${path}.position`),
    tied: boolean(data.tied, `${path}.tied`),
    player_id: uuid(data.player_id, `${path}.player_id`),
    display_name: string(data.display_name, `${path}.display_name`),
    status: participantStatus(data.status, `${path}.status`),
    completed_rounds: integer(data.completed_rounds, `${path}.completed_rounds`, 0),
    gross_total: integer(data.gross_total, `${path}.gross_total`),
    net_total: integer(data.net_total, `${path}.net_total`),
    current_team: currentTeam(data.current_team, `${path}.current_team`),
  }
}

export function decodeRoundLeaderboard(
  value: unknown,
  expectedRoundId: string,
  expectedMetric: LeaderboardMetric,
): RoundLeaderboard {
  const data = object(value, 'leaderboard')
  const decoded: RoundLeaderboard = {
    round_id: uuid(data.round_id, 'leaderboard.round_id'),
    tournament_id: uuid(data.tournament_id, 'leaderboard.tournament_id'),
    status: roundStatus(data.status, 'leaderboard.status'),
    scoring_format: scoringFormat(data.scoring_format, 'leaderboard.scoring_format'),
    metric: metric(data.metric, 'leaderboard.metric'),
    number_of_holes: integer(data.number_of_holes, 'leaderboard.number_of_holes', 1),
    entries: array(data.entries, 'leaderboard.entries', roundEntry),
  }
  if (decoded.round_id !== expectedRoundId || decoded.metric !== expectedMetric) invalid('leaderboard.identity')
  return decoded
}

export function decodeTournamentLeaderboard(
  value: unknown,
  expectedTournamentId: string,
  expectedMetric: LeaderboardMetric,
): TournamentLeaderboard {
  const data = object(value, 'leaderboard')
  const decoded: TournamentLeaderboard = {
    tournament_id: uuid(data.tournament_id, 'leaderboard.tournament_id'),
    metric: metric(data.metric, 'leaderboard.metric'),
    current_round_id: data.current_round_id === null ? null : uuid(data.current_round_id, 'leaderboard.current_round_id'),
    included_round_ids: array(data.included_round_ids, 'leaderboard.included_round_ids', uuid),
    entries: array(data.entries, 'leaderboard.entries', tournamentEntry),
  }
  if (decoded.tournament_id !== expectedTournamentId || decoded.metric !== expectedMetric) invalid('leaderboard.identity')
  return decoded
}
