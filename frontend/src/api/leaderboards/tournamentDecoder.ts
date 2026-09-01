import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeString, decodeUuid } from '../decoder'
import type {
  CurrentTeam,
  LeaderboardMetric,
  TournamentContribution,
  TournamentLeaderboard,
  TournamentLeaderboardEntry,
} from '../types'
import { decodeScoreVisibility } from '../visibility'
import { decodeLeaderboardOwner, decodeMetric, decodeParticipantStatus, invalidLeaderboard, nullablePosition } from './shared'

function currentTeam(value: unknown, path: string): CurrentTeam | null {
  if (value === null) return null
  const data = decodeObject(value, path, 'resultatdata')
  return {
    round_id: decodeUuid(data.round_id, `${path}.round_id`, 'resultatdata'),
    team_id: decodeUuid(data.team_id, `${path}.team_id`, 'resultatdata'),
    team_name: decodeString(data.team_name, `${path}.team_name`, 'resultatdata'),
  }
}

function contribution(value: unknown, path: string): TournamentContribution {
  const data = decodeObject(value, path, 'resultatdata')
  return {
    round_id: decodeUuid(data.round_id, `${path}.round_id`, 'resultatdata'),
    owner: decodeLeaderboardOwner(data.owner, `${path}.owner`),
    owner_name: decodeString(data.owner_name, `${path}.owner_name`, 'resultatdata'),
    provisional: decodeBoolean(data.provisional, `${path}.provisional`, 'resultatdata'),
    holes_scored: decodeInteger(data.holes_scored, `${path}.holes_scored`, 0, undefined, 'resultatdata'),
    number_of_holes: decodeInteger(data.number_of_holes, `${path}.number_of_holes`, 1, undefined, 'resultatdata'),
    gross_total: decodeInteger(data.gross_total, `${path}.gross_total`, undefined, undefined, 'resultatdata'),
    net_total: decodeInteger(data.net_total, `${path}.net_total`, undefined, undefined, 'resultatdata'),
    par_total: decodeInteger(data.par_total, `${path}.par_total`, undefined, undefined, 'resultatdata'),
    score_to_par: decodeInteger(data.score_to_par, `${path}.score_to_par`, undefined, undefined, 'resultatdata'),
    counted: decodeBoolean(data.counted, `${path}.counted`, 'resultatdata'),
    mandatory: decodeBoolean(data.mandatory, `${path}.mandatory`, 'resultatdata'),
  }
}

function tournamentEntry(value: unknown, path: string): TournamentLeaderboardEntry {
  const data = decodeObject(value, path, 'resultatdata')
  return {
    position: nullablePosition(data.position, `${path}.position`),
    tied: decodeBoolean(data.tied, `${path}.tied`, 'resultatdata'),
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'resultatdata'),
    display_name: decodeString(data.display_name, `${path}.display_name`, 'resultatdata'),
    status: decodeParticipantStatus(data.status, `${path}.status`),
    completed_rounds: decodeInteger(data.completed_rounds, `${path}.completed_rounds`, 0, undefined, 'resultatdata'),
    counted_contributions: decodeInteger(data.counted_contributions, `${path}.counted_contributions`, 0, undefined, 'resultatdata'),
    eligible: decodeBoolean(data.eligible, `${path}.eligible`, 'resultatdata'),
    gross_total: decodeInteger(data.gross_total, `${path}.gross_total`, undefined, undefined, 'resultatdata'),
    net_total: decodeInteger(data.net_total, `${path}.net_total`, undefined, undefined, 'resultatdata'),
    par_total: decodeInteger(data.par_total, `${path}.par_total`, undefined, undefined, 'resultatdata'),
    score_to_par: decodeInteger(data.score_to_par, `${path}.score_to_par`, undefined, undefined, 'resultatdata'),
    contributions: decodeArray(data.contributions, `${path}.contributions`, contribution, 'resultatdata'),
    current_team: currentTeam(data.current_team, `${path}.current_team`),
  }
}

function metricScore(item: TournamentContribution, metric: LeaderboardMetric): number {
  return (metric === 'gross' ? item.gross_total : item.net_total) - item.par_total
}

function validateContribution(
  item: TournamentContribution,
  path: string,
  entry: TournamentLeaderboardEntry,
  leaderboard: TournamentLeaderboard,
  included: Set<string>,
): void {
  if (item.holes_scored > item.number_of_holes) invalidLeaderboard(`${path}.holes_scored`)
  if (item.provisional) {
    if (item.holes_scored === 0
      || leaderboard.current_round_id === null
      || item.round_id !== leaderboard.current_round_id
      || included.has(item.round_id)) invalidLeaderboard(`${path}.round_id`)
  } else {
    if (!included.has(item.round_id)) invalidLeaderboard(`${path}.round_id`)
    if (item.holes_scored !== item.number_of_holes) invalidLeaderboard(`${path}.holes_scored`)
  }
  if (item.mandatory !== (item.round_id === leaderboard.mandatory_round_id)) {
    invalidLeaderboard(`${path}.mandatory`)
  }
  if (item.owner.type === 'player' && item.owner.id !== entry.player_id) {
    invalidLeaderboard(`${path}.owner.id`)
  }
  if (item.score_to_par !== metricScore(item, leaderboard.metric)) {
    invalidLeaderboard(`${path}.score_to_par`)
  }
}

function completedQualification(
  contributions: TournamentContribution[],
  leaderboard: TournamentLeaderboard,
): { counted: number; eligible: boolean } {
  const completed = contributions.filter((item) => !item.provisional)
  const mandatoryCompleted = leaderboard.mandatory_round_id === null
    || completed.some((item) => item.mandatory)
  const mandatoryCount = leaderboard.mandatory_round_id !== null && mandatoryCompleted ? 1 : 0
  const optionalSlots = leaderboard.required_counted_rounds - Number(leaderboard.mandatory_round_id !== null)
  const optionalCount = Math.min(optionalSlots, completed.filter((item) => !item.mandatory).length)
  const counted = mandatoryCount + optionalCount
  return { counted, eligible: counted === leaderboard.required_counted_rounds && mandatoryCompleted }
}

function validateEntry(
  entry: TournamentLeaderboardEntry,
  path: string,
  leaderboard: TournamentLeaderboard,
  included: Set<string>,
): void {
  const rounds = new Set<string>()
  let provisionalCount = 0
  entry.contributions.forEach((item, index) => {
    const contributionPath = `${path}.contributions[${index}]`
    if (rounds.has(item.round_id)) invalidLeaderboard(`${contributionPath}.round_id`)
    rounds.add(item.round_id)
    provisionalCount += Number(item.provisional)
    validateContribution(item, contributionPath, entry, leaderboard, included)
  })
  if (provisionalCount > 1) invalidLeaderboard(`${path}.contributions`)
  if (entry.completed_rounds !== entry.contributions.length - provisionalCount) {
    invalidLeaderboard(`${path}.completed_rounds`)
  }

  const qualification = completedQualification(entry.contributions, leaderboard)
  if (entry.counted_contributions !== qualification.counted) invalidLeaderboard(`${path}.counted_contributions`)
  if (entry.eligible !== qualification.eligible) invalidLeaderboard(`${path}.eligible`)

  const selected = entry.contributions.filter((item) => item.counted)
  if (selected.length > leaderboard.required_counted_rounds
    || selected.length < qualification.counted
    || selected.length > qualification.counted + 1) invalidLeaderboard(`${path}.counted_contributions`)
  const mandatory = entry.contributions.find((item) => item.mandatory)
  if (mandatory !== undefined && !mandatory.counted) invalidLeaderboard(`${path}.counted_contributions`)
  const optionalSlots = leaderboard.required_counted_rounds - Number(leaderboard.mandatory_round_id !== null)
  const selectedOptional = selected.filter((item) => !item.mandatory)
  const optional = entry.contributions.filter((item) => !item.mandatory)
  if (selectedOptional.length !== Math.min(optionalSlots, optional.length)) invalidLeaderboard(`${path}.counted_contributions`)
  const rejectedOptional = optional.filter((item) => !item.counted)
  if (selectedOptional.some((selectedItem) => rejectedOptional.some((rejected) =>
    metricScore(selectedItem, leaderboard.metric) > metricScore(rejected, leaderboard.metric)))) {
    invalidLeaderboard(`${path}.contributions`)
  }

  const provisional = entry.contributions.find((item) => item.provisional)
  if (provisional?.owner.type === 'team'
    && (entry.current_team === null
      || entry.current_team.team_id !== provisional.owner.id
      || entry.current_team.team_name !== provisional.owner_name)) invalidLeaderboard(`${path}.current_team`)

  const totals = selected.reduce((sum, item) => ({
    gross: sum.gross + item.gross_total,
    net: sum.net + item.net_total,
    par: sum.par + item.par_total,
  }), { gross: 0, net: 0, par: 0 })
  if (entry.gross_total !== totals.gross) invalidLeaderboard(`${path}.gross_total`)
  if (entry.net_total !== totals.net) invalidLeaderboard(`${path}.net_total`)
  if (entry.par_total !== totals.par) invalidLeaderboard(`${path}.par_total`)
  const total = leaderboard.metric === 'gross' ? entry.gross_total : entry.net_total
  if (entry.score_to_par !== total - entry.par_total) invalidLeaderboard(`${path}.score_to_par`)
}

function selectedProgress(entry: TournamentLeaderboardEntry): number {
  return entry.contributions
    .filter((item) => item.counted && item.provisional)
    .reduce((sum, item) => sum + item.holes_scored, 0)
}

function validateRanks(entries: TournamentLeaderboardEntry[]): void {
  let rankedCount = 0
  for (const entry of entries) {
    if (entry.contributions.some((item) => item.counted)) rankedCount += 1
    else break
  }
  if (entries.slice(rankedCount).some((entry) => entry.position !== null || entry.tied)) {
    invalidLeaderboard('leaderboard.entries.position')
  }
  for (let index = 0; index < rankedCount; index += 1) {
    const entry = entries[index]
    if (entry === undefined) invalidLeaderboard('leaderboard.entries.position')
    const previous = entries[index - 1]
    const next = entries[index + 1]
    const samePrevious = previous !== undefined
      && previous.counted_contributions === entry.counted_contributions
      && previous.score_to_par === entry.score_to_par
    const sameNext = index + 1 < rankedCount && next !== undefined
      && next.counted_contributions === entry.counted_contributions
      && next.score_to_par === entry.score_to_par
    const expectedPosition = samePrevious ? previous.position : index + 1
    if (entry.position !== expectedPosition || entry.tied !== (samePrevious || sameNext)) {
      invalidLeaderboard(`leaderboard.entries[${index}].position`)
    }
    if (previous !== undefined && (previous.counted_contributions < entry.counted_contributions
      || (previous.counted_contributions === entry.counted_contributions
        && (previous.score_to_par > entry.score_to_par
          || (previous.score_to_par === entry.score_to_par
            && selectedProgress(previous) < selectedProgress(entry)))))) {
      invalidLeaderboard(`leaderboard.entries[${index}].position`)
    }
  }
}

function validateCoherence(leaderboard: TournamentLeaderboard): void {
  const included = new Set(leaderboard.included_round_ids)
  if (included.size !== leaderboard.included_round_ids.length) invalidLeaderboard('leaderboard.included_round_ids')
  if (leaderboard.current_round_id !== null && included.has(leaderboard.current_round_id)) {
    invalidLeaderboard('leaderboard.current_round_id')
  }
  const players = new Set<string>()
  leaderboard.entries.forEach((entry, index) => {
    const path = `leaderboard.entries[${index}]`
    if (players.has(entry.player_id)) invalidLeaderboard(`${path}.player_id`)
    players.add(entry.player_id)
    if (entry.current_team !== null
      && (leaderboard.current_round_id === null || entry.current_team.round_id !== leaderboard.current_round_id)) {
      invalidLeaderboard(`${path}.current_team.round_id`)
    }
    validateEntry(entry, path, leaderboard, included)
  })
  validateRanks(leaderboard.entries)
}

export function decodeTournamentLeaderboard(
  value: unknown,
  expectedTournamentId: string,
  expectedMetric: LeaderboardMetric,
): TournamentLeaderboard {
  const data = decodeObject(value, 'leaderboard', 'resultatdata')
  const decoded: TournamentLeaderboard = {
    tournament_id: decodeUuid(data.tournament_id, 'leaderboard.tournament_id', 'resultatdata'),
    metric: decodeMetric(data.metric, 'leaderboard.metric'),
    required_counted_rounds: decodeInteger(data.required_counted_rounds, 'leaderboard.required_counted_rounds', 1, undefined, 'resultatdata'),
    mandatory_round_id: data.mandatory_round_id === null ? null : decodeUuid(data.mandatory_round_id, 'leaderboard.mandatory_round_id', 'resultatdata'),
    current_round_id: data.current_round_id === null ? null : decodeUuid(data.current_round_id, 'leaderboard.current_round_id', 'resultatdata'),
    included_round_ids: decodeArray(data.included_round_ids, 'leaderboard.included_round_ids', (item, path) => decodeUuid(item, path, 'resultatdata'), 'resultatdata'),
    visibility: decodeScoreVisibility(data.visibility, 'leaderboard.visibility', 'resultatdata'),
    entries: decodeArray(data.entries, 'leaderboard.entries', tournamentEntry, 'resultatdata'),
  }
  if (decoded.tournament_id !== expectedTournamentId || decoded.metric !== expectedMetric) {
    invalidLeaderboard('leaderboard.identity')
  }
  validateCoherence(decoded)
  return decoded
}
