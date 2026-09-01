import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeString, decodeUuid } from '../decoder'
import type { CurrentTeam, LeaderboardMetric, TournamentContribution, TournamentLeaderboard, TournamentLeaderboardEntry } from '../types'
import { decodeScoreVisibility } from '../visibility'
import { decodeLeaderboardOwner, decodeMetric, decodeParticipantStatus, invalidLeaderboard, nullablePosition } from './shared'

function currentTeam(value: unknown, path: string): CurrentTeam | null {
  if (value === null) return null
  const data = decodeObject(value, path, 'resultatdata')
  return { round_id: decodeUuid(data.round_id, `${path}.round_id`, 'resultatdata'), team_id: decodeUuid(data.team_id, `${path}.team_id`, 'resultatdata'), team_name: decodeString(data.team_name, `${path}.team_name`, 'resultatdata') }
}

function contribution(value: unknown, path: string): TournamentContribution {
  const data = decodeObject(value, path, 'resultatdata')
  return {
    round_id: decodeUuid(data.round_id, `${path}.round_id`, 'resultatdata'), owner: decodeLeaderboardOwner(data.owner, `${path}.owner`),
    owner_name: decodeString(data.owner_name, `${path}.owner_name`, 'resultatdata'),
    gross_total: decodeInteger(data.gross_total, `${path}.gross_total`, undefined, undefined, 'resultatdata'),
    net_total: decodeInteger(data.net_total, `${path}.net_total`, undefined, undefined, 'resultatdata'),
    par_total: decodeInteger(data.par_total, `${path}.par_total`, undefined, undefined, 'resultatdata'),
    score_to_par: decodeInteger(data.score_to_par, `${path}.score_to_par`, undefined, undefined, 'resultatdata'),
    counted: decodeBoolean(data.counted, `${path}.counted`, 'resultatdata'), mandatory: decodeBoolean(data.mandatory, `${path}.mandatory`, 'resultatdata'),
  }
}

function tournamentEntry(value: unknown, path: string): TournamentLeaderboardEntry {
  const data = decodeObject(value, path, 'resultatdata')
  return {
    position: nullablePosition(data.position, `${path}.position`), tied: decodeBoolean(data.tied, `${path}.tied`, 'resultatdata'),
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'resultatdata'), display_name: decodeString(data.display_name, `${path}.display_name`, 'resultatdata'),
    status: decodeParticipantStatus(data.status, `${path}.status`), completed_rounds: decodeInteger(data.completed_rounds, `${path}.completed_rounds`, 0, undefined, 'resultatdata'),
    counted_contributions: decodeInteger(data.counted_contributions, `${path}.counted_contributions`, 0, undefined, 'resultatdata'),
    eligible: decodeBoolean(data.eligible, `${path}.eligible`, 'resultatdata'), gross_total: decodeInteger(data.gross_total, `${path}.gross_total`, undefined, undefined, 'resultatdata'),
    net_total: decodeInteger(data.net_total, `${path}.net_total`, undefined, undefined, 'resultatdata'), par_total: decodeInteger(data.par_total, `${path}.par_total`, undefined, undefined, 'resultatdata'),
    score_to_par: decodeInteger(data.score_to_par, `${path}.score_to_par`, undefined, undefined, 'resultatdata'),
    contributions: decodeArray(data.contributions, `${path}.contributions`, contribution, 'resultatdata'), current_team: currentTeam(data.current_team, `${path}.current_team`),
  }
}

function validateCoherence(leaderboard: TournamentLeaderboard): void {
  const included = new Set(leaderboard.included_round_ids)
  if (included.size !== leaderboard.included_round_ids.length) invalidLeaderboard('leaderboard.included_round_ids')
  if (leaderboard.current_round_id !== null && included.has(leaderboard.current_round_id)) invalidLeaderboard('leaderboard.current_round_id')
  const players = new Set<string>()
  leaderboard.entries.forEach((entry, index) => {
    const path = `leaderboard.entries[${index}]`
    if (players.has(entry.player_id)) invalidLeaderboard(`${path}.player_id`); players.add(entry.player_id)
    if (entry.current_team !== null && (leaderboard.current_round_id === null || entry.current_team.round_id !== leaderboard.current_round_id)) invalidLeaderboard(`${path}.current_team.round_id`)
    if (entry.contributions.length !== entry.completed_rounds) invalidLeaderboard(`${path}.completed_rounds`)
    const counted = entry.contributions.filter((item) => item.counted)
    if (counted.length !== entry.counted_contributions || counted.length > leaderboard.required_counted_rounds) invalidLeaderboard(`${path}.counted_contributions`)
    const mandatory = entry.contributions.find((item) => item.mandatory)
    if ((mandatory !== undefined && !mandatory.counted) || (leaderboard.mandatory_round_id !== null && mandatory === undefined && entry.counted_contributions >= leaderboard.required_counted_rounds)) invalidLeaderboard(`${path}.counted_contributions`)
    if (entry.eligible !== (entry.counted_contributions === leaderboard.required_counted_rounds)) invalidLeaderboard(`${path}.eligible`)
    const contributionRounds = new Set<string>()
    entry.contributions.forEach((item, contributionIndex) => {
      const contributionPath = `${path}.contributions[${contributionIndex}]`
      if (contributionRounds.has(item.round_id) || !included.has(item.round_id)) invalidLeaderboard(`${contributionPath}.round_id`)
      contributionRounds.add(item.round_id)
      if (item.mandatory !== (item.round_id === leaderboard.mandatory_round_id)) invalidLeaderboard(`${contributionPath}.mandatory`)
      if (item.owner.type === 'player' && item.owner.id !== entry.player_id) invalidLeaderboard(`${contributionPath}.owner.id`)
      const total = leaderboard.metric === 'gross' ? item.gross_total : item.net_total
      if (item.score_to_par !== total - item.par_total) invalidLeaderboard(`${contributionPath}.score_to_par`)
    })
    const totals = counted.reduce((sum, item) => ({ gross: sum.gross + item.gross_total, net: sum.net + item.net_total, par: sum.par + item.par_total }), { gross: 0, net: 0, par: 0 })
    if (entry.gross_total !== totals.gross) invalidLeaderboard(`${path}.gross_total`)
    if (entry.net_total !== totals.net) invalidLeaderboard(`${path}.net_total`)
    if (entry.par_total !== totals.par) invalidLeaderboard(`${path}.par_total`)
    const total = leaderboard.metric === 'gross' ? entry.gross_total : entry.net_total
    if (entry.score_to_par !== total - entry.par_total) invalidLeaderboard(`${path}.score_to_par`)
  })
}

export function decodeTournamentLeaderboard(value: unknown, expectedTournamentId: string, expectedMetric: LeaderboardMetric): TournamentLeaderboard {
  const data = decodeObject(value, 'leaderboard', 'resultatdata')
  const decoded: TournamentLeaderboard = {
    tournament_id: decodeUuid(data.tournament_id, 'leaderboard.tournament_id', 'resultatdata'), metric: decodeMetric(data.metric, 'leaderboard.metric'),
    required_counted_rounds: decodeInteger(data.required_counted_rounds, 'leaderboard.required_counted_rounds', 1, undefined, 'resultatdata'),
    mandatory_round_id: data.mandatory_round_id === null ? null : decodeUuid(data.mandatory_round_id, 'leaderboard.mandatory_round_id', 'resultatdata'),
    current_round_id: data.current_round_id === null ? null : decodeUuid(data.current_round_id, 'leaderboard.current_round_id', 'resultatdata'),
    included_round_ids: decodeArray(data.included_round_ids, 'leaderboard.included_round_ids', (item, path) => decodeUuid(item, path, 'resultatdata'), 'resultatdata'),
    visibility: decodeScoreVisibility(data.visibility, 'leaderboard.visibility', 'resultatdata'), entries: decodeArray(data.entries, 'leaderboard.entries', tournamentEntry, 'resultatdata'),
  }
  if (decoded.tournament_id !== expectedTournamentId || decoded.metric !== expectedMetric) invalidLeaderboard('leaderboard.identity')
  validateCoherence(decoded)
  return decoded
}
