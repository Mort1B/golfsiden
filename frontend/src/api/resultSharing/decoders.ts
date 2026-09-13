import { overallSelected, overallTieBreak } from '../leaderboards/values'
import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeString, decodeTimestamp, decodeUuid, invalidData } from '../decoder'
import { decodeMetric, nullablePosition } from '../leaderboards/shared'
import { decodeTieBreakPolicy } from '../tieBreakPolicy'
import type { LeaderboardMetric } from '../types'
import { decodeScoreVisibility } from '../visibility'
import type { PublicResults, PublicStanding, ResultShareGrant, ResultShareReceipt, ResultShareStatus } from './contracts'

function exact(data: Record<string, unknown>, names: string[], path: string): void {
  if (Object.keys(data).some(key => !names.includes(key)) || names.some(key => !(key in data))) invalidData('delte resultater', path)
}
function timestamp(value: unknown, path: string): string {
  const result = decodeTimestamp(value, path)
  if (!Number.isFinite(Date.parse(result))) invalidData('delte resultater', path)
  return result
}
export function decodeShareGrant(value: unknown): ResultShareGrant {
  const data = decodeObject(value, 'grant')
  exact(data, ['id', 'created_at', 'expires_at', 'revoked_at'], 'grant')
  const grant = { id: decodeUuid(data.id, 'grant.id'), created_at: timestamp(data.created_at, 'grant.created_at'),
    expires_at: timestamp(data.expires_at, 'grant.expires_at'), revoked_at: data.revoked_at === null ? null : timestamp(data.revoked_at, 'grant.revoked_at') }
  if (Date.parse(grant.expires_at) <= Date.parse(grant.created_at)
    || (grant.revoked_at !== null && Date.parse(grant.revoked_at) < Date.parse(grant.created_at))) invalidData('lenkedata', 'grant dates')
  return grant
}
export function decodeShareStatus(value: unknown, tournamentId: string): ResultShareStatus {
  const data = decodeObject(value, 'result_share')
  exact(data, ['tournament_id', 'grant'], 'result_share')
  const id = decodeUuid(data.tournament_id, 'result_share.tournament_id')
  if (id !== tournamentId) invalidData('lenkedata', 'tournament identity')
  return { tournament_id: id, grant: data.grant === null ? null : decodeShareGrant(data.grant) }
}
export function decodeShareReceipt(value: unknown, tournamentId: string): ResultShareReceipt {
  const data = decodeObject(value, 'result_share')
  exact(data, ['tournament_id', 'grant', 'token'], 'result_share')
  const status = decodeShareStatus({ tournament_id: data.tournament_id, grant: data.grant }, tournamentId)
  const token = decodeString(data.token, 'result_share.token')
  if (!status.grant || status.grant.revoked_at !== null || !/^[A-Za-z0-9_-]{43}$/.test(token)) invalidData('lenkedata', 'receipt')
  return { tournament_id: tournamentId, grant: status.grant, token }
}
function publicValue(value: unknown, path: string): NonNullable<PublicStanding['value']> {
  const data = decodeObject(value, path)
  exact(data, ['type', 'version', 'selected', 'tie_break'], path)
  if (data.type !== 'overall_equivalent' || data.version !== 1) invalidData('delte resultater', path)
  return { type: 'overall_equivalent', version: 1, selected: decodeInteger(data.selected, path), tie_break: data.tie_break === null ? null : decodeInteger(data.tie_break, path) }
}
function entry(value: unknown, path: string): PublicStanding {
  const data = decodeObject(value, path)
  exact(data, ['position', 'tied', 'display_name', 'completed_rounds', 'counted_contributions', 'eligible', ...(data.value === undefined ? ['total', 'par_total', 'score_to_par', 'tie_break_score_to_par'] : ['value']), 'provisional', 'provisional_holes_scored'], path)
  return {
    position: nullablePosition(data.position, `${path}.position`), tied: decodeBoolean(data.tied, `${path}.tied`),
    display_name: decodeString(data.display_name, `${path}.display_name`),
    completed_rounds: decodeInteger(data.completed_rounds, `${path}.completed_rounds`, 0, 30),
    counted_contributions: decodeInteger(data.counted_contributions, `${path}.counted_contributions`, 0, 30),
    eligible: decodeBoolean(data.eligible, `${path}.eligible`),
    ...(data.value === undefined ? { total: decodeInteger(data.total, `${path}.total`),
    par_total: decodeInteger(data.par_total, `${path}.par_total`, 0), score_to_par: decodeInteger(data.score_to_par, `${path}.score_to_par`),
    tie_break_score_to_par: data.tie_break_score_to_par === null ? null : decodeInteger(data.tie_break_score_to_par, `${path}.tie_break_score_to_par`),
    } : { value: publicValue(data.value, path) }),
    provisional: decodeBoolean(data.provisional, `${path}.provisional`),
    provisional_holes_scored: decodeInteger(data.provisional_holes_scored, `${path}.provisional_holes_scored`, 0, 18),

  }
}
function primary(left: PublicStanding, right: PublicStanding): boolean {
  return left.position !== null && right.position !== null
    && left.counted_contributions === right.counted_contributions && overallSelected(left) === overallSelected(right)
}
function coherence(board: PublicResults): void {
  const fail = () => invalidData('delte resultater', 'standings coherence')
  let unranked = false
  board.entries.forEach((item, index, all) => {
    if (item.completed_rounds > board.final_round_number || item.counted_contributions > board.required_counted_rounds
      || item.counted_contributions > item.completed_rounds || item.eligible !== (item.counted_contributions === board.required_counted_rounds)
      || (item.value === undefined && item.total - item.par_total !== overallSelected(item)) || item.provisional !== (item.provisional_holes_scored > 0)) fail()
    if (item.position === null) {
      unranked = true
      if (item.tied || (item.value ? item.value.selected !== 0 : item.total !== 0 || item.par_total !== 0) || item.counted_contributions !== 0
        || item.provisional || overallTieBreak(item) !== null) fail()
      return
    }
    if (unranked) fail()
    const previous = all[index - 1]; const next = all[index + 1]
    const samePrevious = previous !== undefined && primary(previous, item)
    const sameNext = next !== undefined && primary(item, next)
    if (overallTieBreak(item) !== null && (board.tie_break_policy !== 'final_round_score'
      || board.visibility.mode !== 'full' || !item.eligible || item.provisional || (!samePrevious && !sameNext))) fail()
    if (samePrevious && previous && (overallTieBreak(previous) === null) !== (overallTieBreak(item) === null)) fail()
    const tiedPrevious = samePrevious && (previous ? overallTieBreak(previous) : undefined) === overallTieBreak(item)
    const tiedNext = sameNext && (next ? overallTieBreak(next) : undefined) === overallTieBreak(item)
    if (item.position !== (tiedPrevious ? previous?.position : index + 1) || item.tied !== (tiedPrevious || tiedNext)) fail()
    if (previous && (previous.counted_contributions < item.counted_contributions
      || (previous.counted_contributions === item.counted_contributions && (overallSelected(previous) > overallSelected(item)
        || (samePrevious && overallTieBreak(previous) !== null && overallTieBreak(item) !== null
          && (overallTieBreak(previous) ?? 0) > (overallTieBreak(item) ?? 0)))))) fail()
  })
}
export function decodePublicResults(value: unknown, grantId: string, metric: LeaderboardMetric): PublicResults {
  const data = decodeObject(value, 'results')
  exact(data, ['grant_id', 'expires_at', 'tournament_name', 'metric', 'required_counted_rounds', 'final_round_number', 'tie_break_policy', 'visibility', 'entries'], 'results')
  const final = decodeInteger(data.final_round_number, 'results.final_round_number', 1, 30)
  const board: PublicResults = {
    grant_id: decodeUuid(data.grant_id, 'results.grant_id'), expires_at: timestamp(data.expires_at, 'results.expires_at'),
    tournament_name: decodeString(data.tournament_name, 'results.tournament_name'), metric: decodeMetric(data.metric, 'results.metric'),
    required_counted_rounds: decodeInteger(data.required_counted_rounds, 'results.required_counted_rounds', 1, final), final_round_number: final,
    tie_break_policy: decodeTieBreakPolicy(data.tie_break_policy, 'results.tie_break_policy', 'delte resultater'),
    visibility: decodeScoreVisibility(data.visibility, 'results.visibility', 'delte resultater'), entries: decodeArray(data.entries, 'results.entries', entry),
  }
  exact(decodeObject(data.visibility, 'results.visibility'), ['mode'], 'results.visibility')
  if (board.grant_id !== grantId || board.metric !== metric) invalidData('delte resultater', 'identity')
  if (new Set(board.entries.map(item => item.value?.type ?? 'stroke')).size > 1) invalidData('delte resultater', 'value basis')
  coherence(board)
  return board
}
