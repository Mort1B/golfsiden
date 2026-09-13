import { useState } from 'react'
import type { MatchScoringCard } from '../../api/matchPlay'
import { useAuth } from '../auth/authContext'
import { useMatchQueue } from './offline/context'
import { matchDatabase } from './offline/database'
import { matchDraftKey, type MatchTarget } from './offline/model'
import { strokes } from './format'
export function MatchNotes({ card, disabled, onUnsafe, recoveryOnly = false }: { card: MatchScoringCard; disabled: boolean; recoveryOnly?: boolean; onUnsafe: (value: boolean) => void }) {
  const queue = useMatchQueue(), { session } = useAuth()
  const [hole, setHole] = useState(Math.min(card.resolved_holes + 1, 18)), [edits, setEdits] = useState<Record<string, string>>({}), [busy, setBusy] = useState(false), [error, setError] = useState<string | null>(null)
  const target: MatchTarget = { accountId: session?.user_id ?? '', tournamentId: card.tournament_id, roundId: card.round_id, matchId: card.match_id }
  const item = queue.items.find(i => i.key === matchDraftKey(target))
  const currentHole = card.holes.find(h => h.hole_number === hole)
  const set = (player: string, value: string) => { setEdits({ ...edits, [player]: value }); onUnsafe(true); setError(null) }
  const save = async (player: string) => {
    const value = edits[player]; if (value === undefined || !session || busy) return
    const gross = Number(value)
    if (value !== '' && (!Number.isInteger(gross) || gross < 1 || gross > 20)) { setError('Bruk 1–20 slag, eller tøm feltet for å fjerne notatet.'); return }
    setBusy(true); setError(null)
    try {
      const command = value === '' ? { type: 'clear_note' as const, player_id: player, hole_number: hole } : { type: 'note' as const, player_id: player, hole_number: hole, gross_strokes: gross }
      await matchDatabase.enqueue(target, item?.generation ?? null, card.revision, command, card.notes.find(n => n.player_id === player && n.hole_number === hole)?.gross_strokes ?? null)
      if (!queue.runtime.isCurrent()) return
      const remaining = { ...edits }; delete remaining[player]; setEdits(remaining); onUnsafe(Object.keys(remaining).length > 0)
      await queue.runtime.changed()
    } catch (e) { setError(e instanceof Error ? e.message : 'Lagring mislyktes.') } finally { setBusy(false) }
  }
  const local = Object.keys(edits).length > 0
  if (recoveryOnly) return local ? <section className="match-panel"><h3>Ulagrede lokale notater</h3><p>Tilgangen er endret. Notatene er beholdt på denne siden, men kan ikke sendes.</p>{Object.values(edits).map((value, i) => <p key={i}>Lokalt notat {i + 1}: {value || 'tomt'}</p>)}<button type="button" disabled={busy} onClick={() => { setEdits({}); setError(null); onUnsafe(false) }}>Forkast ulagrede notater</button></section> : null
  return <section className="match-panel"><h3>Numeriske notater · ikke rapporterte resultater</h3>
    <label>Hull<select disabled={local || busy} value={hole} onChange={e => setHole(Number(e.target.value))}>{card.holes.map(h => <option key={h.hole_number} value={h.hole_number}>Hull {h.hole_number} · par {h.par} · indeks {h.stroke_index}</option>)}</select></label>
    {card.opponents.map((p, i) => {
      const pending = item?.notes.filter(n => (n.request.command.type === 'note' || n.request.command.type === 'clear_note') && n.request.command.player_id === p.player_id && n.request.command.hole_number === hole).at(-1)
      const command = pending?.request.command
      const pendingValue = command?.type === 'note' ? String(command.gross_strokes) : command?.type === 'clear_note' ? '' : undefined
      const value = edits[p.player_id] ?? pendingValue ?? String(card.notes.find(n => n.player_id === p.player_id && n.hole_number === hole)?.gross_strokes ?? '')
      return <div key={p.player_id}><label>Notat · {p.display_name}<input type="number" inputMode="numeric" min={1} max={20} value={value} disabled={disabled || busy || queue.loading || !!queue.error} onChange={e => set(p.player_id, e.target.value)} /></label>
        <p>{currentHole ? strokes(card.relative_handicaps[i] ?? 0, currentHole.stroke_index) : 0} relative handicapslag · {pending ? 'Lokalt notat venter på levering/kontroll' : 'Gjeldende servernotat'}</p>
        <button type="button" disabled={disabled || busy || edits[p.player_id] === undefined} onClick={() => void save(p.player_id)}>{busy ? 'Lagrer …' : 'Lagre notat'}</button></div>
    })}
    {local && <button type="button" disabled={busy} onClick={() => { setEdits({}); setError(null); onUnsafe(false) }}>Forkast ulagrede notater</button>}
    {(error || queue.error) && <p role="alert">{error ?? queue.error}</p>}
    {!queue.online && <p role="status">Frakoblet. Notater lagres på enheten; rapportering og bekreftelse venter på nett.</p>}
  </section>
}
