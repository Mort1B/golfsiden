import { useState } from 'react'
import type { MatchScoringCard } from '../../api/matchPlay'
import { useAuth } from '../auth/authContext'
import { useMatchQueue } from './offline/context'
import { matchDraftKey, type MatchTarget } from './offline/model'
import { useMatchNoteDrafts } from './drafts/context'
import { MatchNoteRecovery } from './drafts/MatchNoteRecovery'
import { strokes } from './format'
export function MatchNotes({ card, disabled, recoveryOnly = false }: { card: MatchScoringCard; disabled: boolean; recoveryOnly?: boolean }) {
  const queue = useMatchQueue(), { session } = useAuth(), { store, drafts } = useMatchNoteDrafts()
  const [selectedHole, setHole] = useState(Math.min(card.resolved_holes + 1, 18))
  const local = drafts.filter(item => item.target.roundId === card.round_id && item.target.matchId === card.match_id)
  const hole = local[0]?.hole ?? selectedHole, busy = local.some(item => item.saving)
  const target: MatchTarget = { accountId: session?.user_id ?? '', tournamentId: card.tournament_id, roundId: card.round_id, matchId: card.match_id }
  const item = queue.items.find(i => i.key === matchDraftKey(target))
  const currentHole = card.holes.find(h => h.hole_number === hole)
  if (recoveryOnly || local.some(draft => !card.opponents.some(p => p.player_id === draft.playerId))) return <MatchNoteRecovery roundId={card.round_id} matchId={card.match_id} />
  return <section className="match-panel"><h3>Numeriske notater · ikke rapporterte resultater</h3>
    <label>Hull<select disabled={local.length > 0 || busy} value={hole} onChange={e => setHole(Number(e.target.value))}>{card.holes.map(h => <option key={h.hole_number} value={h.hole_number}>Hull {h.hole_number} · par {h.par} · indeks {h.stroke_index}</option>)}</select></label>
    {card.opponents.map((p, i) => {
      const draft = local.find(d => d.playerId === p.player_id && d.hole === hole)
      const pending = item?.notes.filter(n => (n.request.command.type === 'note' || n.request.command.type === 'clear_note') && n.request.command.player_id === p.player_id && n.request.command.hole_number === hole).at(-1)
      const command = pending?.request.command
      const pendingValue = command?.type === 'note' ? String(command.gross_strokes) : command?.type === 'clear_note' ? '' : undefined
      const oldValue = card.notes.find(n => n.player_id === p.player_id && n.hole_number === hole)?.gross_strokes ?? null
      const value = draft?.value ?? pendingValue ?? String(oldValue ?? '')
      return <div key={p.player_id}><label>Notat · {p.display_name}<input type="number" inputMode="numeric" min={1} max={20} value={value} disabled={disabled || busy || queue.loading || !!queue.error}
        onChange={e => store.set({ target, playerId: p.player_id, slot: i + 1, hole, value: e.target.value, revision: card.revision, observed: item?.generation ?? null, oldValue }, queue.runtime)} /></label>
        <p>{currentHole ? strokes(card.relative_handicaps[i] ?? 0, currentHole.stroke_index) : 0} relative handicapslag · {draft ? 'Ikke lagret på enheten' : pending ? 'Lokalt notat venter på levering/kontroll' : 'Gjeldende servernotat'}</p>
        <button type="button" disabled={disabled || busy || !draft} onClick={() => { if (draft) void store.save(draft.key, queue.runtime) }}>{busy ? 'Lagrer …' : 'Lagre notat'}</button>
        {draft?.error && <p role="alert">{draft.error}</p>}
      </div>
    })}
    {local.length > 0 && <button type="button" disabled={busy} onClick={() => store.discard(card.round_id, card.match_id)}>Forkast ulagrede notater</button>}
    {queue.error && <p role="alert">{queue.error}</p>}
    {!queue.online && <p role="status">Frakoblet. Notater lagres på enheten; rapportering og bekreftelse venter på nett.</p>}
  </section>
}
