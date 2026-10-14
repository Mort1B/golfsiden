import { decodeArray, decodeBoolean, decodeObject, decodeString, decodeTimestamp, decodeUuid, invalidData } from '../decoder'
import { awardTypes, type AwardType, type AwardReceipt, type AwardsView } from './types'
function invalid(path: string): never { return invalidData('rundeutmerkelser', path) }
export function decodeAwardType(value: unknown, path: string): AwardType {
  if (value === 'closest_to_pin' || value === 'longest_drive') return value
  return invalid(path)
}
export function decodeRevision(value: unknown, path: string): string {
  const text = decodeString(value, path)
  if (!/^(0|[1-9][0-9]*)$/.test(text) || BigInt(text) > 9223372036854775807n) invalid(path)
  return text
}
const nullableUuid = (value: unknown, path: string) => value === null ? null : decodeUuid(value, path)
export function decodeAwardReceipt(value: unknown, path = 'receipt'): AwardReceipt {
  const d = decodeObject(value, path)
  const result = { request_id: decodeUuid(d.request_id, path), round_id: decodeUuid(d.round_id, path), award_type: decodeAwardType(d.award_type, path), player_id: nullableUuid(d.player_id, path), previous_player_id: nullableUuid(d.previous_player_id, path), revision: decodeRevision(d.revision, path), reason: decodeString(d.reason, path), correction: decodeBoolean(d.correction, path), actor_id: decodeUuid(d.actor_id, path), created_at: decodeTimestamp(d.created_at, path) }
  if (result.revision === '0' || !validAwardReason(result.reason)) invalid(path)
  return result
}
export function validAwardReason(reason: string): boolean { return reason.trim().length > 0 && new TextEncoder().encode(reason).length <= 500 && !reason.includes('\0') }
export function decodeAwardsView(value: unknown, round: string, tournament: string): AwardsView {
  const d = decodeObject(value, 'awards')
  const availability = d.availability
  if (availability !== 'available' && availability !== 'round_not_completed' && availability !== 'unsupported_format') invalid('availability')
  const view: AwardsView = { round_id: decodeUuid(d.round_id, 'round'), tournament_id: decodeUuid(d.tournament_id, 'tournament'), availability, correction_required: decodeBoolean(d.correction_required, 'correction'),
    eligible_players: decodeArray(d.eligible_players, 'players', (v,p) => { const x=decodeObject(v,p); return { player_id: decodeUuid(x.player_id,p), display_name: decodeString(x.display_name,p) } }),
    awards: decodeArray(d.awards, 'slots', (v,p) => { const x=decodeObject(v,p); return { award_type: decodeAwardType(x.award_type,p), player_id: nullableUuid(x.player_id,p), revision: decodeRevision(x.revision,p) } }),
    history: decodeArray(d.history, 'history', decodeAwardReceipt) }
  if (view.round_id !== round || view.tournament_id !== tournament) invalid('identity')
  const players = new Set(view.eligible_players.map(p=>p.player_id))
  if (players.size !== view.eligible_players.length || view.awards.length !== 2 || awardTypes.some(type=>view.awards.filter(a=>a.award_type===type).length!==1)) invalid('inventory')
  if (view.awards.some(a=>a.player_id!==null && (!players.has(a.player_id)||a.revision==='0')) || view.history.some(r=>r.round_id!==round || r.player_id!==null&&!players.has(r.player_id) || r.previous_player_id!==null&&!players.has(r.previous_player_id))) invalid('participant')
  if (new Set(view.history.map(r=>r.request_id)).size !== view.history.length) invalid('history')
  return view
}
