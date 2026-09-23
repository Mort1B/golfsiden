import type { MatchNoteCommand } from '../../../api/matchPlay'
import { matchDatabase } from '../offline/database'
import { matchDraftKey, type MatchTarget } from '../offline/model'
import type { MatchRuntime } from '../offline/runtime'

export interface MatchNoteIntent {
  target: MatchTarget
  playerId: string
  slot: number
  hole: number
  value: string
  revision: string
  observed: string | null
  oldValue: number | null
}
export interface MatchNoteDraft extends MatchNoteIntent { key: string; sequence: number; saving: boolean; error: string | null }
const keyOf = (intent: MatchNoteIntent) => `${matchDraftKey(intent.target)}:${intent.hole}:${intent.playerId}`

// Account-owned transient intent only: no canonical cards, names or authority.
export class MatchNoteStore {
  private drafts: MatchNoteDraft[] = []
  private listeners = new Set<() => void>()
  private sequence = 0
  private active = true
  constructor(readonly accountId: string) {}
  current = () => this.drafts
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener) } }
  private publish(drafts: MatchNoteDraft[]) { this.drafts = drafts; this.listeners.forEach(listener => listener()) }
  activate = () => { this.active = true }
  close = () => { this.active = false; this.publish([]) }
  set = (intent: MatchNoteIntent, runtime: MatchRuntime) => {
    if (!this.active || intent.target.accountId !== this.accountId || !runtime.isCurrent()) return
    const key = keyOf(intent), previous = this.drafts.find(item => item.key === key)
    if (this.drafts.some(item => item.target.matchId === intent.target.matchId && item.saving)) return
    this.publish([...this.drafts.filter(item => item.key !== key), {
      ...(previous ?? intent), value: intent.value, key, sequence: ++this.sequence, saving: false, error: null,
    }])
  }
  discard = (roundId: string, matchId: string) => {
    if (this.drafts.some(item => item.target.roundId === roundId && item.target.matchId === matchId && item.saving)) return
    this.publish(this.drafts.filter(item => item.target.roundId !== roundId || item.target.matchId !== matchId))
  }
  save = async (key: string, runtime: MatchRuntime) => {
    const draft = this.drafts.find(item => item.key === key)
    if (!this.active || !draft || runtime.accountId !== this.accountId || !runtime.isCurrent()
      || this.drafts.some(item => item.target.matchId === draft.target.matchId && item.saving)) return
    const gross = Number(draft.value)
    if (draft.value !== '' && (!Number.isInteger(gross) || gross < 1 || gross > 20)) {
      this.publish(this.drafts.map(item => item.key === key ? { ...item, error: 'Bruk 1–20 slag, eller tøm feltet for å fjerne notatet.' } : item))
      return
    }
    this.publish(this.drafts.map(item => item.key === key ? { ...item, saving: true, error: null } : item))
    try {
      const command: MatchNoteCommand = draft.value === ''
        ? { type: 'clear_note', player_id: draft.playerId, hole_number: draft.hole }
        : { type: 'note', player_id: draft.playerId, hole_number: draft.hole, gross_strokes: gross }
      const saved = await matchDatabase.enqueue(draft.target, draft.observed, draft.revision, command, draft.oldValue,
        { allowed: () => this.active, subscribe: this.subscribe })
      if (!this.active) return
      // Committed retention survives session replacement. Only our own append
      // advances sibling expectations; an unrelated queue refresh never does.
      this.publish(this.drafts.filter(item => item.key !== key || item.sequence !== draft.sequence).map(item =>
        item.target.matchId === draft.target.matchId && item.observed === draft.observed ? { ...item, observed: saved.generation } : item))
      if (runtime.isCurrent()) await runtime.changed()
    } catch (error) {
      if (!this.active) return
      this.publish(this.drafts.map(item => item.key === key && item.sequence === draft.sequence
        ? { ...item, saving: false, error: error instanceof Error ? error.message : 'Lagring mislyktes.' } : item))
    }
  }
}
