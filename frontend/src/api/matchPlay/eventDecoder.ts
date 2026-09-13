import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeString, decodeUuid, invalidData } from '../decoder'
import { decodeScoreRevision } from '../scorecards/revision'
import type { MatchBasis, MatchEvent, MatchOutcome, MatchRequest, MatchCommand } from './contracts'
export function exact(data: Record<string, unknown>, fields: string[]): void {
  if (Object.keys(data).some(k => !fields.includes(k)) || fields.some(k => !(k in data))) invalidData('matchdata', 'fields')
}
export function reason(value: unknown): string {
  const text = decodeString(value, 'reason')
  if (!text.trim() || new TextEncoder().encode(text).length > 1000 || text.includes('\0')) invalidData('matchdata', 'reason')
  return text
}
export function outcome(value: unknown): MatchOutcome {
  if (value !== 'first' && value !== 'second' && value !== 'halved') invalidData('matchdata', 'outcome')
  return value
}
export function decodeBasis(value: unknown): MatchBasis {
  const d = decodeObject(value, 'basis')
  if (d.type === 'numeric' || d.type === 'next_stroke_concession') {
    exact(d, ['type', 'first_gross', 'second_gross', 'agreed', ...(d.type === 'numeric' ? [] : ['conceding_player_id', 'communicated'])])
    const scores = { first_gross: decodeInteger(d.first_gross, 'first_gross', 1, 20), second_gross: decodeInteger(d.second_gross, 'second_gross', 1, 20), agreed: decodeBoolean(d.agreed, 'agreed') }
    return d.type === 'numeric' ? { type: d.type, ...scores } : { type: d.type, ...scores,
      conceding_player_id: decodeUuid(d.conceding_player_id, 'conceder'), communicated: decodeBoolean(d.communicated, 'communicated') }
  }
  if (d.type === 'hole_concession') {
    exact(d, ['type', 'conceding_player_id', 'communicated'])
    return { type: d.type, conceding_player_id: decodeUuid(d.conceding_player_id, 'conceder'), communicated: decodeBoolean(d.communicated, 'communicated') }
  }
  if (d.type === 'agreed_halve') {
    exact(d, ['type', 'play_begun', 'mutual_agreement'])
    return { type: d.type, play_begun: decodeBoolean(d.play_begun, 'play_begun'), mutual_agreement: decodeBoolean(d.mutual_agreement, 'mutual_agreement') }
  }
  if (d.type === 'organizer_ruling') { exact(d, ['type', 'reason']); return { type: d.type, reason: reason(d.reason) } }
  return invalidData('matchdata', 'basis.type')
}
export function decodeEvent(value: unknown): MatchEvent {
  const d = decodeObject(value, 'event')
  if (d.type === 'hole') {
    exact(d, ['type', 'hole_number', 'outcome', 'basis'])
    return { type: d.type, hole_number: decodeInteger(d.hole_number, 'hole', 1, 18), outcome: outcome(d.outcome), basis: decodeBasis(d.basis) }
  }
  if (d.type === 'concession') {
    exact(d, ['type', 'conceding_player_id', 'communicated', 'after_hole'])
    return { type: d.type, conceding_player_id: decodeUuid(d.conceding_player_id, 'conceder'), communicated: decodeBoolean(d.communicated, 'communicated'), after_hole: decodeInteger(d.after_hole, 'after_hole', 0, 18) }
  }
  if (d.type === 'award') {
    exact(d, ['type', 'winner_player_id', 'reason', 'after_hole'])
    return { type: d.type, winner_player_id: decodeUuid(d.winner_player_id, 'winner'), reason: reason(d.reason), after_hole: decodeInteger(d.after_hole, 'after_hole', 0, 18) }
  }
  return invalidData('matchdata', 'event.type')
}
export function decodeCommand(value: unknown): MatchCommand {
  const d = decodeObject(value, 'command')
  if (d.type === 'note' || d.type === 'clear_note') {
    exact(d, ['type', 'player_id', 'hole_number', ...(d.type === 'note' ? ['gross_strokes'] : [])])
    const target = { player_id: decodeUuid(d.player_id, 'player'), hole_number: decodeInteger(d.hole_number, 'hole', 1, 18) }
    return d.type === 'note' ? { type: d.type, ...target, gross_strokes: decodeInteger(d.gross_strokes, 'gross', 1, 20) } : { type: d.type, ...target }
  }
  if (d.type === 'report') { exact(d, ['type', 'event']); return { type: d.type, event: decodeEvent(d.event) } }
  if (d.type === 'confirm') { exact(d, ['type', 'result_agreed_or_awarded']); return { type: d.type, result_agreed_or_awarded: decodeBoolean(d.result_agreed_or_awarded, 'agreed') } }
  if (d.type === 'correct') {
    exact(d, ['type', 'kind', 'reason', 'superseded_event_ids', 'replacement'])
    if (d.kind !== 'recording_error' && d.kind !== 'organizer_ruling') invalidData('matchdata', 'kind')
    return { type: d.type, kind: d.kind, reason: reason(d.reason), superseded_event_ids: decodeArray(d.superseded_event_ids, 'superseded', decodeUuid), replacement: decodeArray(d.replacement, 'replacement', decodeEvent) }
  }
  return invalidData('matchdata', 'command')
}
export function decodeRequest(value: unknown): MatchRequest {
  const d = decodeObject(value, 'request'); exact(d, ['request_id', 'expected_revision', 'command'])
  return { request_id: decodeUuid(d.request_id, 'request_id'), expected_revision: decodeScoreRevision(d.expected_revision), command: decodeCommand(d.command) }
}
