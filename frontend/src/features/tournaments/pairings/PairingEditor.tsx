import { AlertTriangle, LockKeyhole, RefreshCw, Save } from 'lucide-react'
import type { PairingDraftGroup, PairingDraft } from './draft'
import type { Round } from '../../../api/types'
import { legacyConversionReplacement, replacementFromDraft, scheduleFlightOptions, selectScheduleFlight, validateDraft } from './draft'
import { pairingFailureMessage, usePairingEditor } from './usePairingEditor'
import { GroupEditor } from './GroupEditor'

interface Props { tournamentId: string; round: Round; expanded: boolean }

function LegacyConversion({ groups, disabled, onConvert }: { groups: Parameters<typeof legacyConversionReplacement>[0]['legacy_individual_groups']; disabled: boolean; onConvert: () => void }) {
  return <div className="pairing-legacy" role="status"><AlertTriangle aria-hidden="true" /><div><strong>Eldre individuelle grupper må konverteres</strong><p>Første lagring kopierer navn, spillere, rekkefølge og eventuell startplan nøyaktig til flighter. Vanlig redigering åpnes etterpå.</p><ul>{groups.map((group) => <li key={group.id}><strong>{group.name}</strong><span>{group.members.map((member) => member.display_name).join(', ') || 'Ingen spillere'}</span></li>)}</ul><button type="button" disabled={disabled} onClick={onConvert}>{disabled ? 'Konverterer…' : 'Konverter grupper og lagre'}</button></div></div>
}

function ScheduleTransfers({ draft, disabled, onChange }: { draft: PairingDraft; disabled: boolean; onChange: (update: (draft: PairingDraft) => PairingDraft) => void }) {
  const scheduled = draft.teams.filter((team) => team.requiresScheduleTransfer)
  if (scheduled.length === 0) return null
  const updateTeam = (teamId: string, scheduleFlightId: string | null) =>
    onChange((current) => selectScheduleFlight(current, teamId, scheduleFlightId))
  return <fieldset className="pairing-transfers" disabled={disabled}><legend>Overfør gamle lagstarter eksplisitt</legend><p>Likhet brukes aldri som automatisk kobling. Når du velger en flight med nøyaktig samme spillere, kopieres lagets starthull og utslagstid til flighten.</p>{scheduled.map((team: PairingDraftGroup) => {
    const options = scheduleFlightOptions(team, draft.flights)
    const schedule = [team.startingHole && `hull ${team.startingHole}`, team.teeTime && `kl. ${team.teeTime}`].filter(Boolean).join(' · ')
    return <label key={team.id}><span>{team.name}{schedule ? ` (${schedule})` : ''}</span><select value={team.scheduleFlightId ?? ''} onChange={(event) => updateTeam(team.id, event.target.value || null)}><option value="">Velg nøyaktig flight</option>{options.map((flight) => <option key={flight.id} value={flight.id}>{flight.name}</option>)}</select>{options.length === 0 && <small>Opprett en flight med nøyaktig de samme spillerne.</small>}</label>
  })}</fieldset>
}

export function PairingEditor({ tournamentId, round, expanded }: Props) {
  const state = usePairingEditor({ tournamentId, round, expanded })
  const pairings = state.query.data
  if (state.query.isPending) return <p className="pairing-read-state">Laster spillegrupper…</p>
  if (state.query.error) return <div className="pairing-read-error" role="alert"><p>Spillegruppene kunne ikke lastes.</p><button type="button" onClick={() => void state.query.refetch()}><RefreshCw aria-hidden="true" /> Prøv igjen</button></div>
  if (!pairings || !state.draft) return <p className="pairing-read-state">Ingen spillegruppedata er tilgjengelig.</p>
  const draft = state.draft
  const saving = state.mutation.isPending
  const locked = pairings.status !== 'draft'
  const hasLegacy = pairings.legacy_individual_groups.length > 0
  const validation = validateDraft(draft, pairings)
  const save = () => {
    if (validation.blocking.length > 0 || locked || state.reloadConflict) return
    void state.save(replacementFromDraft(draft, pairings.scoring_format))
  }
  const convert = () => void state.save(legacyConversionReplacement(pairings))
  return <div className="pairing-editor">
    {locked && <p className="pairing-locked"><LockKeyhole aria-hidden="true" /> Runden er {pairings.status}. Bare utkast kan redigeres; lagret oppsett vises under.</p>}
    {state.reloadConflict && <div className="pairing-conflict" role="alert"><AlertTriangle aria-hidden="true" /><div><strong>En nyere versjon finnes</strong><p>Ditt lokale utkast er beholdt og blir ikke overskrevet. Forkast det for å laste siste lagrede oppsett.</p><button type="button" disabled={saving} onClick={() => void state.discardAndReload()}><RefreshCw aria-hidden="true" /> Forkast og last på nytt</button></div></div>}
    {pairingFailureMessage(state.failure) && <p className="pairing-save-error" role="alert">{pairingFailureMessage(state.failure)}</p>}
    {hasLegacy ? <LegacyConversion groups={pairings.legacy_individual_groups} disabled={saving || locked || state.reloadConflict} onConvert={convert} /> : <>
      {pairings.active_entrants.length === 0 && <p className="pairing-empty">Ingen aktive deltakere kan tildeles i denne runden.</p>}
      {pairings.scoring_format === 'team_scramble' && <GroupEditor kind="team" idScope={round.id} draft={draft} entrants={pairings.active_entrants} inactiveEntrants={pairings.inactive_entrants} disabled={saving || locked || state.reloadConflict} onChange={state.edit} />}
      <GroupEditor kind="flight" idScope={round.id} draft={draft} entrants={pairings.active_entrants} inactiveEntrants={pairings.inactive_entrants} disabled={saving || locked || state.reloadConflict} onChange={state.edit} />
      {pairings.scoring_format === 'team_scramble' && <ScheduleTransfers draft={draft} disabled={saving || locked || state.reloadConflict} onChange={state.edit} />}
      {pairings.inactive_entrants.length > 0 && <div className="pairing-inactive"><strong>Inaktive deltakere</strong><p>Disse kan ikke legges til på nytt: {pairings.inactive_entrants.map((entrant) => entrant.display_name).join(', ')}.</p></div>}
      {validation.unresolved.length > 0 && <div className="pairing-unresolved" role="status"><strong>Ikke klart for åpning</strong><ul>{validation.unresolved.map((issue) => <li key={issue}>{issue}</li>)}</ul><p>Ufullstendige oppsett kan lagres. Backendens åpningskontroll er autoritativ.</p></div>}
      {validation.blocking.length > 0 && <div className="pairing-validation" role="alert"><strong>Rett før lagring</strong><ul>{validation.blocking.map((issue) => <li key={issue}>{issue}</li>)}</ul></div>}
      <div className="pairing-save-row"><span aria-live="polite">{saving ? 'Lagrer…' : state.dirty ? 'Ulagrede endringer' : 'Synkronisert med serveren'}</span><button type="button" disabled={saving || locked || !state.dirty || state.reloadConflict || validation.blocking.length > 0} onClick={save}><Save aria-hidden="true" /> {saving ? 'Lagrer…' : 'Lagre hele oppsettet'}</button></div>
    </>}
  </div>
}
