import { useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { useTournamentArchive, type ArchiveProps } from './useTournamentArchive'
import './archive.css'

export function TournamentArchivePanel(props: ArchiveProps) {
  const state = useTournamentArchive(props)
  const [confirmation, setConfirmation] = useState<string | null>(null)
  const actionRef = useRef<HTMLButtonElement>(null)
  const cancelRef = useRef<HTMLButtonElement>(null)
  const headingRef = useRef<HTMLHeadingElement>(null)
  const messageRef = useRef<HTMLParagraphElement>(null)
  const wasConfirming = useRef(false)
  const version = JSON.stringify([props.snapshotVersion, props.tournament.updated_at])
  const confirming = confirmation !== null && confirmation === version && state.canAct
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
  const archived = props.tournament.status === 'archived'
  return <section className="tournament-archive" aria-labelledby="tournament-archive-heading" aria-busy={state.busy}>
    <h3 id="tournament-archive-heading" ref={headingRef} tabIndex={-1}>{archived ? 'Turneringen er arkivert' : 'Arkiver turneringen'}</h3>
    {state.busy && <p role="status">Kontrollerer og oppdaterer turneringen …</p>}
    {!state.busy && props.authorityRefreshing && <p role="status">Kontrollerer status og tilgang …</p>}
    {state.message && <p ref={messageRef} tabIndex={-1} role={state.message.error ? 'alert' : 'status'}>{state.message.text}</p>}
    {state.refreshFailed && <p role="alert">Oppdateringen mislyktes. Arkivering er stengt til status og tilgang er kontrollert på nytt.</p>}
    <button type="button" disabled={state.busy || props.authorityRefreshing} onClick={() => { setConfirmation(null); void state.refresh() }}>Oppdater arkiveringskontrollen</button>
    {archived ? <p>Turneringen finnes i arkivet. Eksisterende medlemmer beholder tilgangen til historikk og resultater.</p> : <>
      {props.tournament.status !== 'completed' ? <p>Fullfør turneringen før den arkiveres.</p> : <p>Turneringen er fullført og kan arkiveres.</p>}
      <button ref={actionRef} type="button" className="archive-action" disabled={!state.canAct || confirming} onClick={() => setConfirmation(version)}>Arkiver turneringen</button>
      {confirming && <fieldset onKeyDown={(event) => { if (event.key === 'Escape') { event.preventDefault(); setConfirmation(null) } }}>
        <legend>Bekreft arkivering av turneringen</legend>
        <p>Arkivere «{props.tournament.name}»? Turneringen flyttes fra Nåværende til Arkiv. Historikk og resultater slettes ikke, og eksisterende medlemmer beholder tilgangen. Arkivering kan ikke angres.</p>
        <p>Synligheten av finalens siste ni hull endres ikke.</p>
        <div><button ref={cancelRef} type="button" onClick={() => setConfirmation(null)}>Avbryt</button>
          <button type="button" className="archive-action" onClick={() => { setConfirmation(null); void state.archive() }}>Bekreft og arkiver turneringen</button></div>
      </fieldset>}
    </>}
    <Link to="/tournaments?view=archived">Se turneringsarkivet</Link>
    <p>Finalens siste ni hull styres fortsatt av den separate synlighetskontrollen.</p>
  </section>
}
