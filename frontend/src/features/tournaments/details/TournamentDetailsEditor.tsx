import { Save, RefreshCw, LockKeyhole } from 'lucide-react'
import { useAuth } from '../../auth/authContext'
import { useTournamentDetails, type DetailsProps } from './useTournamentDetails'
import './tournamentDetails.css'
export function TournamentDetailsEditor(props: DetailsProps) {
  const { session } = useAuth()
  if (!session) return null
  return <DetailsForm key={`${session.user_id}:${session.csrf_token}:${props.tournament.id}`} {...props} />
}
function DetailsForm(props: DetailsProps) {
  const editor = useTournamentDetails(props)
  if (props.tournament.status !== 'draft') return <p className="tournament-details-locked"><LockKeyhole aria-hidden="true" />Navn, beskrivelse og datoer kan bare endres mens turneringen er et utkast.</p>
  return <form className="tournament-details-editor" aria-label="Turneringsopplysninger" aria-busy={editor.busy}
    onSubmit={event => { event.preventDefault(); void editor.save() }}>
    <h3>Navn og datoer</h3>
    <p>Endre turneringsopplysningene før start. Rundedatoer og spilleoppsett beholdes.</p>
    {props.loading && <p role="status">Henter oppdatert turnering og rundeplan …</p>}
    {props.readError && <div role="alert"><p>Kunne ikke hente rundeplanen.</p><button type="button" onClick={props.retry}>Prøv igjen</button></div>}
    <fieldset disabled={editor.disabled}>
      <label htmlFor="tournament-detail-name">Turneringsnavn</label>
      <input id="tournament-detail-name" value={editor.fields.name} onChange={event => editor.change('name', event.target.value)}
        aria-invalid={!!editor.errors.name} aria-describedby={editor.errors.name ? 'tournament-detail-name-error' : undefined} />
      {editor.errors.name && <p id="tournament-detail-name-error" className="details-error">{editor.errors.name}</p>}
      <label htmlFor="tournament-detail-description">Beskrivelse (valgfritt)</label>
      <textarea id="tournament-detail-description" rows={3} value={editor.fields.description} onChange={event => editor.change('description', event.target.value)}
        aria-invalid={!!editor.errors.description} aria-describedby={editor.errors.description ? 'tournament-detail-description-error' : undefined} />
      {editor.errors.description && <p id="tournament-detail-description-error" className="details-error">{editor.errors.description}</p>}
      <div className="details-dates">{(['start_date', 'end_date'] as const).map(key => <div key={key}>
        <label htmlFor={`tournament-detail-${key}`}>{key === 'start_date' ? 'Startdato' : 'Sluttdato'}</label>
        <input type="date" id={`tournament-detail-${key}`} value={editor.fields[key]} onChange={event => editor.change(key, event.target.value)}
          aria-invalid={!!editor.errors[key]} aria-describedby={editor.errors[key] ? `tournament-detail-${key}-error` : undefined} />
        {editor.errors[key] && <p id={`tournament-detail-${key}-error`} className="details-error">{editor.errors[key]}</p>}
      </div>)}</div>
    </fieldset>
    {editor.conflict && <div role="alert"><p>Kontroller siste serverversjon før du lagrer igjen. Ditt utkast er beholdt.</p>
      <button type="button" disabled={editor.busy} onClick={() => void editor.reload()}><RefreshCw aria-hidden="true" />Forkast utkast og hent siste</button></div>}
    {editor.failure && <p role="alert" className="details-error">{editor.failure}</p>}
    <button className="details-save" type="submit" disabled={editor.disabled || editor.unchanged || editor.conflict || Object.keys(editor.errors).length > 0}>
      <Save aria-hidden="true" />{editor.busy ? 'Lagrer …' : editor.failure ? 'Prøv lagring igjen' : 'Lagre turneringsopplysninger'}</button>
    {editor.receipt && <p role="status">Turneringsopplysningene er lagret og kontrollert.</p>}
  </form>
}
