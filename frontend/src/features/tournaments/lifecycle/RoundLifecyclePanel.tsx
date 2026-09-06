import { useEffect, useRef, useState } from 'react'
import { CheckCircle2, LockKeyhole, RefreshCw } from 'lucide-react'
import type { Round, Tournament } from '../../../api/types'
import { StatusBadge } from '../../../ui/StatusBadge'
import { LoadingState } from '../../../ui/AsyncState'
import { roundAction, transitionExplanations, transitionLabels } from './lifecycleState'
import { CompletionReadiness, OpeningReadiness } from './RoundReadiness'
import { useRoundLifecycle } from './useRoundLifecycle'

interface Props {
  tournament: Tournament
  rounds: Round[]
  selectedRoundId: string | null
  onSelectRound: (id: string) => void
  authorityRefreshing: boolean
}

export function RoundLifecyclePanel({ tournament, rounds, selectedRoundId, onSelectRound, authorityRefreshing }: Props) {
  const selected = selectedRoundId === null ? rounds[0] : rounds.find((round) => round.id === selectedRoundId)
  return <div className="round-lifecycle">
    <h3>Administrer en runde</h3>
    <label className="round-lifecycle-selector">Velg runde
      <select value={selected?.id ?? ''} onChange={(event) => onSelectRound(event.target.value)}>
        {!selected && <option value="" disabled>Velg en runde i denne turneringen</option>}
        {rounds.map((round) => <option key={round.id} value={round.id}>Runde {round.round_number}: {round.name}</option>)}
      </select>
    </label>
    {!selected ? <p role="alert">Den valgte runden finnes ikke i denne turneringen.</p>
      : <SelectedRoundLifecycle key={selected.id} tournament={tournament} selectedRound={selected} authorityRefreshing={authorityRefreshing} />}
  </div>
}

export function SelectedRoundLifecycle({ tournament, selectedRound, authorityRefreshing }: { tournament: Tournament; selectedRound: Round; authorityRefreshing: boolean }) {
  const state = useRoundLifecycle(tournament, selectedRound, authorityRefreshing)
  const action = roundAction(state.round.status)
  const [confirmation, setConfirmation] = useState<string | null>(null)
  const actionRef = useRef<HTMLButtonElement>(null)
  const cancelRef = useRef<HTMLButtonElement>(null)
  const receiptRef = useRef<HTMLParagraphElement>(null)
  const headingRef = useRef<HTMLHeadingElement>(null)
  const wasConfirming = useRef(false)
  const confirmationVersion = `${action}:${state.version}`
  const confirming = confirmation === confirmationVersion && state.canAct
  const cancel = () => setConfirmation(null)
  useEffect(() => {
    if (confirming) cancelRef.current?.focus()
    else if (wasConfirming.current) {
      if (actionRef.current && !actionRef.current.disabled) actionRef.current.focus()
      else headingRef.current?.focus()
    }
    wasConfirming.current = confirming
  }, [confirming])
  useEffect(() => {
    if (state.message) receiptRef.current?.focus()
  }, [state.message])

  return <div className="round-lifecycle-detail" aria-busy={state.busy}>
    <header><h4 ref={headingRef} tabIndex={-1}>Runde {state.round.round_number}: {state.round.name}</h4><StatusBadge status={state.round.status} /></header>
    {state.message && <p ref={receiptRef} tabIndex={-1} className={`round-lifecycle-message ${state.message.kind}`} role={state.message.kind === 'error' ? 'alert' : 'status'}>{state.message.text}</p>}
    {state.reconcileFailed && <p role="alert">Oppdateringen mislyktes. Handlinger er stengt til status og tilgang er kontrollert på nytt.</p>}
    {state.error && <p role="alert">Kunne ikke kontrollere rundestatus og krav. Prøv å oppdatere kontrollen.</p>}
    {state.inconsistent && <p role="alert">Rundestatus eller tilgang er endret. Oppdater kontrollen før du fortsetter.</p>}
    <button className="round-lifecycle-refresh" type="button" disabled={state.busy || state.refreshing} onClick={() => { setConfirmation(null); void state.refresh() }}>
      <RefreshCw aria-hidden="true" />{state.busy || state.refreshing ? 'Kontrollerer …' : 'Oppdater kontrollen'}
    </button>
    {state.loading && <LoadingState />}
    {!state.loading && !state.error && !state.inconsistent && <>
      {state.round.status === 'draft' && state.opening && <OpeningReadiness round={state.round} validation={state.opening} />}
      {(state.round.status === 'open' || state.round.status === 'completed') && state.completion && <CompletionReadiness round={state.round} validation={state.completion} />}
    </>}
    {state.round.status === 'locked' ? <p className="round-lifecycle-locked"><LockKeyhole aria-hidden="true" />Runden er låst. Scorekortene er skrivebeskyttet.</p> : action && <>
      <p>{transitionExplanations[action]}</p>
      {state.round.status === 'draft' && tournament.status !== 'active' && <p>Turneringen må være startet før runden kan åpnes.</p>}
      <button ref={actionRef} className="round-lifecycle-action" type="button" disabled={!state.canAct || confirming} onClick={() => setConfirmation(confirmationVersion)}>{transitionLabels[action]}</button>
      {confirming && <fieldset className="round-lifecycle-confirmation" onKeyDown={(event) => { if (event.key === 'Escape') { event.preventDefault(); cancel() } }}>
        <legend>Bekreft: {transitionLabels[action].toLocaleLowerCase('nb-NO')}</legend>
        <p>Dette gjelder runde {state.round.round_number}: {state.round.name}.</p>
        <div className="round-lifecycle-confirmation-actions">
          <button ref={cancelRef} type="button" onClick={cancel}>Avbryt</button>
          <button className="round-lifecycle-action" type="button" onClick={() => { setConfirmation(null); void state.transition(action) }}><CheckCircle2 aria-hidden="true" />Bekreft og {transitionLabels[action].toLocaleLowerCase('nb-NO')}</button>
        </div>
      </fieldset>}
    </>}
    <p className="round-lifecycle-visibility">Fullføring og låsing endrer ikke synligheten av finalens siste ni hull. Visning styres separat.</p>
  </div>
}
