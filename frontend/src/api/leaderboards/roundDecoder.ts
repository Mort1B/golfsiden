import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeString, decodeUuid } from '../decoder'
import type { LeaderboardMetric, RoundLeaderboard, RoundLeaderboardEntry } from '../types'
import { decodeScoreVisibility } from '../visibility'
import { decodeLeaderboardMember, decodeLeaderboardOwner, decodeMetric, decodeRoundStatus, decodeScoringFormat, invalidLeaderboard, nullablePosition } from './shared'

function decodeNullableBoolean(value: unknown, path: string): boolean | null {
  return value === null ? null : decodeBoolean(value, path, 'resultatdata')
}

function roundEntry(value: unknown, path: string): RoundLeaderboardEntry {
  const data = decodeObject(value, path, 'resultatdata')
  return {
    position: nullablePosition(data.position, `${path}.position`), tied: decodeBoolean(data.tied, `${path}.tied`, 'resultatdata'),
    owner: decodeLeaderboardOwner(data.owner, `${path}.owner`), owner_name: decodeString(data.owner_name, `${path}.owner_name`, 'resultatdata'),
    members: decodeArray(data.members, `${path}.members`, decodeLeaderboardMember, 'resultatdata'),
    holes_scored: decodeInteger(data.holes_scored, `${path}.holes_scored`, 0, undefined, 'resultatdata'),
    number_of_holes: decodeInteger(data.number_of_holes, `${path}.number_of_holes`, 1, undefined, 'resultatdata'),
    complete: decodeNullableBoolean(data.complete, `${path}.complete`), confirmed: decodeNullableBoolean(data.confirmed, `${path}.confirmed`),
    playing_handicap: decodeInteger(data.playing_handicap, `${path}.playing_handicap`, undefined, undefined, 'resultatdata'),
    gross_total: decodeInteger(data.gross_total, `${path}.gross_total`, undefined, undefined, 'resultatdata'),
    net_total: decodeInteger(data.net_total, `${path}.net_total`, undefined, undefined, 'resultatdata'),
    par_played: decodeInteger(data.par_played, `${path}.par_played`, 0, undefined, 'resultatdata'),
    score_to_par: decodeInteger(data.score_to_par, `${path}.score_to_par`, undefined, undefined, 'resultatdata'),
  }
}

export function decodeRoundLeaderboard(value: unknown, expectedRoundId: string, expectedTournamentId: string, expectedMetric: LeaderboardMetric): RoundLeaderboard {
  const data = decodeObject(value, 'leaderboard', 'resultatdata')
  const decoded: RoundLeaderboard = {
    round_id: decodeUuid(data.round_id, 'leaderboard.round_id', 'resultatdata'),
    tournament_id: decodeUuid(data.tournament_id, 'leaderboard.tournament_id', 'resultatdata'),
    status: decodeRoundStatus(data.status, 'leaderboard.status'), scoring_format: decodeScoringFormat(data.scoring_format, 'leaderboard.scoring_format'),
    metric: decodeMetric(data.metric, 'leaderboard.metric'), number_of_holes: decodeInteger(data.number_of_holes, 'leaderboard.number_of_holes', 1, undefined, 'resultatdata'),
    visible_hole_count: decodeInteger(data.visible_hole_count, 'leaderboard.visible_hole_count', 1, undefined, 'resultatdata'),
    visibility: decodeScoreVisibility(data.visibility, 'leaderboard.visibility', 'resultatdata'),
    entries: decodeArray(data.entries, 'leaderboard.entries', roundEntry, 'resultatdata'),
  }
  if (decoded.round_id !== expectedRoundId || decoded.tournament_id !== expectedTournamentId || decoded.metric !== expectedMetric) invalidLeaderboard('leaderboard.identity')
  const restricted = decoded.visibility.mode === 'front_nine'
  if ((restricted && (decoded.number_of_holes !== 18 || decoded.visible_hole_count !== 9))
    || (!restricted && decoded.visible_hole_count !== decoded.number_of_holes)) invalidLeaderboard('leaderboard.visibility')
  const ownerIds = new Set<string>()
  decoded.entries.forEach((entry, entryIndex) => {
    const path = `leaderboard.entries[${entryIndex}]`; const ownerId = `${entry.owner.type}:${entry.owner.id}`
    if (ownerIds.has(ownerId)) invalidLeaderboard(`${path}.owner`); ownerIds.add(ownerId)
    const expectedType = decoded.scoring_format === 'individual_stroke_play' ? 'player' : 'team'
    if (entry.owner.type !== expectedType || (entry.owner.type === 'player' && entry.members.length !== 0)
      || (entry.owner.type === 'team' && entry.members.length !== 2)) invalidLeaderboard(`${path}.owner.type`)
    if (entry.number_of_holes !== decoded.number_of_holes || entry.holes_scored > decoded.visible_hole_count) invalidLeaderboard(`${path}.number_of_holes`)
    if (restricted ? entry.complete !== null || entry.confirmed !== null
      : entry.complete === null || entry.confirmed === null || entry.complete !== (entry.holes_scored === entry.number_of_holes) || (entry.confirmed && !entry.complete)) invalidLeaderboard(`${path}.complete`)
    const selected = decoded.metric === 'gross' ? entry.gross_total : entry.net_total
    if (entry.score_to_par !== selected - entry.par_played) invalidLeaderboard(`${path}.score_to_par`)
    if (new Set(entry.members.map((member) => member.player_id)).size !== entry.members.length) invalidLeaderboard(`${path}.members`)
  })
  return decoded
}
