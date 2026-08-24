import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import type { PairingEntrant } from '../../../api/pairings'
import type { DraftGroupKind, PairingDraft, PairingDraftGroup } from './draft'
import { assignEntrant, groupsFor, moveMember, newDraftGroup, replaceGroups } from './draft'

interface Props {
  kind: DraftGroupKind
  draft: PairingDraft
  entrants: PairingEntrant[]
  inactiveEntrants: PairingEntrant[]
  idScope: string
  disabled: boolean
  onChange: (update: (draft: PairingDraft) => PairingDraft) => void
}

function label(kind: DraftGroupKind): string { return kind === 'team' ? 'lag' : 'flight' }

export function GroupEditor({ kind, draft, entrants, inactiveEntrants, idScope, disabled, onChange }: Props) {
  const groups = groupsFor(draft, kind)
  const assignment = new Map(groups.flatMap((group) => group.memberIds.map((id) => [id, group.id] as const)))
  const entrantName = new Map([...entrants, ...inactiveEntrants]
    .map((entrant) => [entrant.player_id, entrant.display_name] as const))
  const activeIds = new Set(entrants.map((entrant) => entrant.player_id))
  const updateGroup = (groupId: string, update: (group: PairingDraftGroup) => PairingDraftGroup) => {
    onChange((current) => replaceGroups(current, kind,
      groupsFor(current, kind).map((group) => group.id === groupId ? update(group) : group)))
  }
  const removeGroup = (groupId: string) => onChange((current) =>
    replaceGroups(current, kind, groupsFor(current, kind).filter((group) => group.id !== groupId)))
  return (
    <section className="pairing-kind" aria-labelledby={`${idScope}-${kind}-heading`}>
      <header><div><h4 id={`${idScope}-${kind}-heading`}>{kind === 'team' ? 'Lag' : 'Flighter'}</h4><p>{kind === 'team' ? 'Lag eier fellesresultatet.' : 'Flighter eier gruppen og startplanen.'}</p></div>
        <button type="button" disabled={disabled} onClick={() => onChange((current) =>
          replaceGroups(current, kind, [...groupsFor(current, kind), newDraftGroup(kind, groupsFor(current, kind).length + 1)]))}>
          <Plus aria-hidden="true" /> Legg til {label(kind)}
        </button>
      </header>
      {groups.length === 0 && <p className="pairing-empty">Ingen {kind === 'team' ? 'lag' : 'flighter'} er opprettet.</p>}
      <div className="pairing-groups">
        {groups.map((group) => (
          <fieldset className="pairing-group" key={group.id} disabled={disabled}>
            <legend>{kind === 'team' ? 'Lag' : 'Flight'}</legend>
            <div className="pairing-group-title">
              <label>Navn<input value={group.name} maxLength={120} onChange={(event) => updateGroup(group.id, (current) => ({ ...current, name: event.target.value }))} /></label>
              <button type="button" className="pairing-remove" onClick={() => removeGroup(group.id)} aria-label={`Fjern ${group.name || label(kind)}`}><Trash2 aria-hidden="true" /> Fjern</button>
            </div>
            {kind === 'flight' && <div className="pairing-schedule">
              <label>Starthull <input type="number" inputMode="numeric" min="1" max="36" value={group.startingHole} onChange={(event) => updateGroup(group.id, (current) => ({ ...current, startingHole: event.target.value }))} /></label>
              <label>Utslagstid <input type="time" value={group.teeTime} onChange={(event) => updateGroup(group.id, (current) => ({ ...current, teeTime: event.target.value, teeTimeEdited: true }))} /></label>
            </div>}
            <ol className="pairing-member-order">
              {group.memberIds.map((playerId, index) => <li key={playerId}><span>{entrantName.get(playerId) ?? 'Ukjent spiller'}{!activeIds.has(playerId) && <small>Inaktiv – må fjernes før lagring</small>}</span><span className="pairing-order-actions">
                {activeIds.has(playerId) ? <><button type="button" disabled={index === 0} onClick={() => onChange((current) => moveMember(current, kind, group.id, playerId, -1))} aria-label={`Flytt ${entrantName.get(playerId) ?? 'spiller'} opp`}><ArrowUp aria-hidden="true" /></button>
                <button type="button" disabled={index === group.memberIds.length - 1} onClick={() => onChange((current) => moveMember(current, kind, group.id, playerId, 1))} aria-label={`Flytt ${entrantName.get(playerId) ?? 'spiller'} ned`}><ArrowDown aria-hidden="true" /></button></> :
                <button type="button" onClick={() => onChange((current) => assignEntrant(current, kind, playerId, null))} aria-label={`Fjern inaktiv spiller ${entrantName.get(playerId) ?? ''}`}><Trash2 aria-hidden="true" /></button>}
              </span></li>)}
            </ol>
          </fieldset>
        ))}
      </div>
      <fieldset className="pairing-assignments" disabled={disabled || groups.length === 0}>
        <legend>Flytt spillere mellom {kind === 'team' ? 'lag' : 'flighter'}</legend>
        {entrants.map((entrant) => <label key={entrant.player_id}><span>{entrant.display_name}</span><select value={assignment.get(entrant.player_id) ?? ''} onChange={(event) => onChange((current) => assignEntrant(current, kind, entrant.player_id, event.target.value || null))}>
          <option value="">Ikke tildelt</option>{groups.map((group) => <option key={group.id} value={group.id}>{group.name || `Uten navn`}</option>)}
        </select></label>)}
      </fieldset>
    </section>
  )
}
