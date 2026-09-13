import type { MatchNoteCommand, MatchRequest } from '../../../api/matchPlay'
import { decodeRequest, exact } from '../../../api/matchPlay/eventDecoder'
import { decodeArray, decodeInteger, decodeObject, decodeString, decodeUuid, invalidData } from '../../../api/decoder'
export interface MatchTarget { accountId: string; tournamentId: string; roundId: string; matchId: string }
export interface Delivery {
  request: MatchRequest; predecessor: string | null; oldValue: number | null
  phase: 'queued' | 'unknown' | 'acknowledged' | 'blocked'; error: string | null
}
export interface MatchDraft extends MatchTarget {
  protocol: 'match_notes_v1'; key: string; generation: string; notes: Delivery[]; action: Delivery | null
  lease: { id: string; until: number } | null; retryAt: number
}
export const matchDraftKey = (target: MatchTarget): string => `${target.accountId}:${target.matchId}`
export const LEASE_MS = 20000
export const STORAGE_ERROR = 'Endringen kunne ikke lagres trygt på enheten. Prøv igjen eller forkast.'
export const STALE_ERROR = 'En annen fane har endret matchen. Se gjennom gammel, lokal og gjeldende verdi før du prøver igjen.'
export function nextRevision(value: string): string {
  const next = BigInt(value) + 1n
  if (next > 9223372036854775807n) throw new Error('Matchens revisjonsgrense er nådd.')
  return String(next)
}
export function noteValue(command: MatchNoteCommand): number | null { return command.type === 'note' ? command.gross_strokes : null }
export function decodeDraft(value: unknown): MatchDraft {
  const d = decodeObject(value, 'match draft')
  exact(d, ['protocol', 'key', 'generation', 'accountId', 'tournamentId', 'roundId', 'matchId', 'notes', 'action', 'lease', 'retryAt'])
  if (d.protocol !== 'match_notes_v1') invalidData('lokale matchdata', 'protocol')
  const target = { accountId: decodeUuid(d.accountId, 'account'), tournamentId: decodeUuid(d.tournamentId, 'tournament'), roundId: decodeUuid(d.roundId, 'round'), matchId: decodeUuid(d.matchId, 'match') }
  if (d.key !== matchDraftKey(target)) invalidData('lokale matchdata', 'key')
  const notes = decodeArray(d.notes, 'notes', decodeDelivery)
  if (notes.some(n => n.request.command.type !== 'note' && n.request.command.type !== 'clear_note')) invalidData('lokale matchdata', 'note command')
  for (let i = 1; i < notes.length; i++) {
    const previous = notes[i - 1], current = notes[i]
    if (!previous || !current || current.predecessor !== previous.request.request_id || current.request.expected_revision !== nextRevision(previous.request.expected_revision)) invalidData('lokale matchdata', 'sequence')
  }
  let lease: MatchDraft['lease'] = null
  if (d.lease !== null) { const l = decodeObject(d.lease, 'lease'); exact(l, ['id', 'until']); lease = { id: decodeUuid(l.id, 'lease.id'), until: decodeInteger(l.until, 'lease.until', 0) } }
  return { ...target, protocol: 'match_notes_v1', key: matchDraftKey(target), generation: decodeUuid(d.generation, 'generation'), notes, action: d.action === null ? null : decodeDelivery(d.action), lease, retryAt: decodeInteger(d.retryAt, 'retry', 0) }
}
function decodeDelivery(value: unknown): Delivery {
  const d = decodeObject(value, 'delivery'); exact(d, ['request', 'predecessor', 'oldValue', 'phase', 'error'])
  if (d.phase !== 'queued' && d.phase !== 'unknown' && d.phase !== 'acknowledged' && d.phase !== 'blocked') invalidData('lokale matchdata', 'phase')
  return { request: decodeRequest(d.request), predecessor: d.predecessor === null ? null : decodeUuid(d.predecessor, 'predecessor'), oldValue: d.oldValue === null ? null : decodeInteger(d.oldValue, 'old', 1, 20), phase: d.phase, error: d.error === null ? null : decodeString(d.error, 'error') }
}
