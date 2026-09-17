import { useEffect } from 'react'
import { useBlocker } from 'react-router-dom'
import { inputLabel } from '../../../api/fourBall'
import { useScoreQueue } from '../offline/context'
import { useScoringGuard } from '../scoringGuardContext'
import { useScoreDrafts } from './context'

export function ScoreRecovery() {
  const { drafts, store } = useScoreDrafts()
  const { runtime } = useScoreQueue()
  const { setBlocked } = useScoringGuard()
  const blocked = drafts.length > 0
  const blocker = useBlocker(blocked)
  useEffect(() => { store.suspend() }, [store])
  useEffect(() => { setBlocked(blocked); return () => setBlocked(false) }, [blocked, setBlocked])
  useEffect(() => { if (blocker.state === 'blocked') blocker.reset() }, [blocker])
  useEffect(() => {
    if (!blocked) return
    const prevent = (event: BeforeUnloadEvent) => { event.preventDefault(); event.returnValue = '' }
    window.addEventListener('beforeunload', prevent)
    return () => window.removeEventListener('beforeunload', prevent)
  }, [blocked])
  return <section className="page score-page" aria-label="Ulagrede scoreendringer">
    <h1>Ta vare på ulagrede scorer</h1>
    <p>Tilgang eller rundestatus kan ikke bekreftes. Her vises bare det du selv har skrevet. Endringene er ikke trygt lagret ennå.</p>
    <p>Du kan lagre en lokal kopi også uten nett. Kopien sendes ikke automatisk: gjeldende tilgang og serverscore må kontrolleres før du velger på nytt.</p>
    {drafts.map((draft, index) => <section className="score-save-error" key={draft.key} aria-label={`Lokal endring ${index + 1}`}>
      <h2>Hull {draft.target.holeNumber} · {draft.kind === 'four_ball' ? `spiller ${draft.slot}` : draft.target.owner.type === 'team' ? 'lagkort' : 'spillerkort'}</h2>
      <p>Din ulagrede score: <strong>{draft.kind === 'legacy' ? `${draft.value} slag` : inputLabel(draft.value)}</strong></p>
      {draft.error && <p role="alert">{draft.error}</p>}
      <div className="pending-actions">
        <button type="button" disabled={draft.saving} onClick={() => store.retry(draft.key, runtime)}>{draft.saving ? 'Lagrer på enheten …' : 'Lagre lokal kopi'}</button>
        <button type="button" disabled={draft.saving} onClick={() => store.discard(draft.key)}>Forkast ulagret endring</button>
      </div>
    </section>)}
  </section>
}
