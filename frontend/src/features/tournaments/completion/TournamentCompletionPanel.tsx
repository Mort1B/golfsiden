import { useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { StatusBadge } from '../../../ui/StatusBadge'
import { LoadingState } from '../../../ui/AsyncState'
import { roundManagementUrl } from '../lifecycle/lifecycleState'
import { useTournamentCompletion, type CompletionProps } from './useTournamentCompletion'
import './completion.css'

export function TournamentCompletionPanel(props: CompletionProps) {
  const state = useTournamentCompletion(props)
  const [confirmation, setConfirmation] = useState<string | null>(null)
  const actionRef = useRef<HTMLButtonElement>(null)
  const cancelRef = useRef<HTMLButtonElement>(null)
  const headingRef = useRef<HTMLHeadingElement>(null)
  const messageRef = useRef<HTMLParagraphElement>(null)
  const wasConfirming = useRef(false)
  const version = JSON.stringify([props.snapshotVersion, props.tournament.updated_at, props.rounds.data])
  const confirming = confirmation !== null && confirmation === version && state.canAct
  // Permanently discard confirmation when a read/authority change invalidates it.
  if (confirmation !== null && !confirming) setConfirmation(null)
  useEffect(() => {
    if (confirming) cancelRef.current?.focus()
    else if (wasConfirming.current) {
      if (actionRef.current && !actionRef.current.disabled) actionRef.current.focus()
      else headingRef.current?.focus()
    }
    wasConfirming.current = confirming
  }, [confirming])
  useEffect(() => { if (state.message) messageRef.current?.focus() }, [state.message])
  const status = props.tournament.status
  const closed = status === 'completed' || status === 'archived'
  return <section className="tournament-completion" aria-labelledby="tournament-completion-heading" aria-busy={state.busy}>
    <h3 id="tournament-completion-heading" ref={headingRef} tabIndex={-1}>{closed ? 'Turneringen er avsluttet' : 'Fullfør turneringen'}</h3>
    {state.busy && <p role="status">Kontrollerer og oppdaterer turneringen …</p>}
    {state.message && <p ref={messageRef} tabIndex={-1} role={state.message.error ? 'alert' : 'status'}>{state.message.text}</p>}
    {state.refreshFailed && <p role="alert">Oppdateringen mislyktes. Fullføring er stengt til status og tilgang er kontrollert på nytt.</p>}
    <button type="button" disabled={state.busy || props.authorityRefreshing} onClick={() => { setConfirmation(null); void state.refresh() }}>Oppdater fullføringskontrollen</button>
    {closed ? <p>Eksisterende medlemmer beholder tilgangen. Nye invitasjoner og påmeldinger er stengt. Turneringen kan ikke åpnes igjen.</p> : <>
      {status === 'draft' && <p>Start turneringen, og fullfør og lås alle rundene før turneringen avsluttes.</p>}
      <p>Alle {props.tournament.number_of_rounds} planlagte runder må være låst, også runder som ikke teller i sammenlagtresultatet.</p>
      {props.rounds.pending ? <LoadingState /> : props.rounds.error ? <p role="alert">Rundeplanen kunne ikke kontrolleres. Prøv å oppdatere kontrollen.</p> : <>
        {!state.readiness.valid && <p role="alert">Rundeplanen er ufullstendig eller ugyldig. Kontroller at alle planlagte runder finnes.</p>}
        {state.readiness.unlocked.length > 0 && <ul aria-label="Runder som må låses">{state.readiness.unlocked.map((round) => <li key={round.id}>
          <Link to={roundManagementUrl(props.tournament.id, round.id)}>Runde {round.round_number}: {round.name}</Link><StatusBadge status={round.status} />
        </li>)}</ul>}
        {state.readiness.ready && <p>Alle rundene er låst. Turneringen er klar til å fullføres.</p>}
      </>}
      <button ref={actionRef} type="button" className="completion-action" disabled={!state.canAct || confirming} onClick={() => setConfirmation(version)}>Fullfør turneringen</button>
      {confirming && <fieldset onKeyDown={(event) => { if (event.key === 'Escape') { event.preventDefault(); setConfirmation(null) } }}>
        <legend>Bekreft fullføring av turneringen</legend>
        <p>Fullføre «{props.tournament.name}»? Nye invitasjoner og påmeldinger stenges. Eksisterende medlemmer beholder tilgangen. Det finnes ingen gjenåpning.</p>
        <p>Synligheten av finalens siste ni hull endres ikke.</p>
        <div><button ref={cancelRef} type="button" onClick={() => setConfirmation(null)}>Avbryt</button>
          <button type="button" className="completion-action" onClick={() => { setConfirmation(null); void state.complete() }}>Bekreft og fullfør turneringen</button></div>
      </fieldset>}
    </>}
    <p>Finalens siste ni hull styres fortsatt av den separate synlighetskontrollen.</p>
  </section>
}
