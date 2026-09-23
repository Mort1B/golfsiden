import { useMatchNoteDrafts } from './drafts/context'
import { useEffect, useState } from 'react'
import type { MatchCommand, MatchScoringCard } from '../../api/matchPlay'
import { useAuth } from '../auth/authContext'
import { useScoringGuard } from '../scoring/scoringGuardContext'
import { useMatchQueue } from './offline/context'
import { matchDraftKey } from './offline/model'
import { MatchReadView } from './MatchRound'
import { MatchNotes } from './MatchNotes'
import { MatchEventForm } from './MatchEventForm'
import { MatchCorrection } from './MatchCorrection'
import { MatchPending } from './MatchPending'
export function MatchScoring({ card, admin, recovering, recoveryOnly = false }: { card: MatchScoringCard; admin: boolean; recovering: boolean; recoveryOnly?: boolean }) {
  const { session } = useAuth(), queue = useMatchQueue(), { drafts } = useMatchNoteDrafts(), [busy, setBusy] = useState(false), [error, setError] = useState<string | null>(null), [agreedRevision, setAgreedRevision] = useState<string | null>(null)
  const target = { accountId: session?.user_id ?? '', tournamentId: card.tournament_id, roundId: card.round_id, matchId: card.match_id }
  const item = queue.items.find(i => i.key === matchDraftKey(target)), pending = !!item && (!!item.action || item.notes.length > 0)
  const unsafe = drafts.some(item => item.target.roundId === card.round_id && item.target.matchId === card.match_id)
  const locked = unsafe || busy, { setBlocked } = useScoringGuard()
  useEffect(() => { setBlocked(locked); return () => setBlocked(false) }, [locked, setBlocked])
  useEffect(() => { if (!locked) return; const warn = (e: BeforeUnloadEvent) => { e.preventDefault(); e.returnValue = '' }; window.addEventListener('beforeunload', warn); return () => window.removeEventListener('beforeunload', warn) }, [locked])
  const disabled = recoveryOnly || busy || unsafe || pending || queue.loading || !!queue.error || !queue.online || recovering || !session
  const submit = async (command: MatchCommand) => {
    if (disabled) return
    setBusy(true); setError(null)
    try { await queue.runtime.onlineAction(target, card, command) } catch (e) { setError(e instanceof Error ? e.message : 'Handlingen kunne ikke kontrolleres.') } finally { setBusy(false) }
  }
  return <>
    {!recoveryOnly && <MatchReadView card={card} />}
    {!recoveryOnly && recovering && <p role="status">Tilgang og gjeldende match kontrolleres. Rapportering venter.</p>}
    {(!recoveryOnly && !card.finish && card.round_status !== 'locked' || unsafe) && <MatchNotes card={card} recoveryOnly={recoveryOnly || !!card.finish || card.round_status === 'locked'} disabled={recoveryOnly || !!card.finish || card.round_status === 'locked' || busy || !!item?.action || item?.notes.some(n => n.phase === 'blocked' || n.phase === 'acknowledged') === true} />}
    {item && pending && <MatchPending item={item} />}
    {disabled && <p role="status">Rapportering og bekreftelse krever nett, oppdatert match og ferdig kontrollerte lokale endringer.</p>}
    {!recoveryOnly && !card.finish && card.round_status !== 'locked' && <MatchEventForm key={`report:${card.revision}`} card={card} admin={admin} disabled={disabled} onSubmit={event => void submit({ type: 'report', event })} />}
    {!recoveryOnly && card.finish && !card.confirmed && (card.round_status !== 'locked' || admin && card.correction_pending) && <section className="match-panel"><h3>Bekreft matchresultatet</h3><label className="match-check"><input type="checkbox" checked={agreedRevision === card.revision} disabled={disabled} onChange={e => setAgreedRevision(e.target.checked ? card.revision : null)} />Jeg bekrefter at resultatet er avtalt av motstanderne eller avgjort av arrangøren.</label><button type="button" disabled={disabled || agreedRevision !== card.revision} onClick={() => void submit({ type: 'confirm', result_agreed_or_awarded: true })}>{busy ? 'Kontrollerer …' : 'Bekreft matchresultat'}</button></section>}
    {!recoveryOnly && admin && <MatchCorrection key={`correction:${card.revision}`} card={card} disabled={disabled} submit={command => void submit(command)} />}
    {error && <p role="alert">{error}</p>}
  </>
}
