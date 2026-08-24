import type { RoundPairings } from '../../../api/pairings'
import { isTeamScoringFormat } from '../../../api/scoringFormats'
import { scheduleFlightOptions, type PairingDraft, type PairingDraftGroup } from './draft'
import { serializedTeeTime } from './serialization'
import type { ScoringFormat } from '../../../api/types'

export interface DraftValidation {
  blocking: string[]
  unresolved: string[]
}

const teamReadinessLabels = {
  individual_stroke_play: null,
  team_scramble: { assignment: 'lag', size: 'lag' },
  two_player_foursomes: { assignment: 'foursomes-lag', size: 'foursomes-lag' },
} satisfies Record<ScoringFormat, { assignment: string; size: string } | null>

function hasDuplicates(values: readonly string[]): boolean {
  return new Set(values).size !== values.length
}

function duplicateNames(groups: PairingDraftGroup[]): boolean {
  return hasDuplicates(groups.map((group) => group.name.trim().toLocaleLowerCase('nb-NO')))
}

function duplicateMembers(groups: PairingDraftGroup[]): boolean {
  return hasDuplicates(groups.flatMap((group) => group.memberIds))
}

export function validateDraft(draft: PairingDraft, pairings: RoundPairings): DraftValidation {
  const blocking: string[] = []
  const unresolved: string[] = []
  const eligible = new Set(pairings.active_entrants.map((entrant) => entrant.player_id))
  const allGroups = [...draft.teams, ...draft.flights]
  if (allGroups.some((group) => group.name.length === 0 || group.name !== group.name.trim())) {
    blocking.push('Alle gruppenavn må være utfylt uten mellomrom først eller sist.')
  }
  if (duplicateNames(draft.teams)) blocking.push('Lagnavn må være unike.')
  if (duplicateNames(draft.flights)) blocking.push('Flightnavn må være unike.')
  if (duplicateMembers(draft.teams)) blocking.push('En spiller kan bare være på ett lag i runden.')
  if (duplicateMembers(draft.flights)) blocking.push('En spiller kan bare være i én flight i runden.')
  if (allGroups.some((group) => group.memberIds.some((id) => !eligible.has(id)))) {
    blocking.push('Oppsettet inneholder en spiller som ikke lenger er aktiv deltaker. Fjern spilleren fra lag og flight før lagring.')
  }
  if (draft.flights.some((flight) => flight.startingHole !== '' &&
    (!/^\d{1,2}$/.test(flight.startingHole) || Number(flight.startingHole) < 1 || Number(flight.startingHole) > 36))) {
    blocking.push('Starthull må være et heltall fra 1 til 36, eller stå tomt.')
  }
  if (draft.flights.some((flight) => flight.teeTime !== '' && !/^([01]\d|2[0-3]):[0-5]\d$/.test(flight.teeTime))) {
    blocking.push('Utslagstid må være et gyldig klokkeslett, eller stå tomt.')
  }
  for (const team of draft.teams.filter((group) => group.requiresScheduleTransfer)) {
    const selected = scheduleFlightOptions(team, draft.flights)
      .find((flight) => flight.id === team.scheduleFlightId)
    if (!selected || selected.startingHole !== team.startingHole ||
      serializedTeeTime(selected) !== serializedTeeTime(team)) {
      blocking.push(`Velg eksplisitt hvilken flight som overtar starttiden fra ${team.name}.`)
    }
  }
  const assignedFlights = new Set(draft.flights.flatMap((flight) => flight.memberIds))
  const unassignedFlights = pairings.active_entrants.filter((entrant) => !assignedFlights.has(entrant.player_id)).length
  if (unassignedFlights > 0) unresolved.push(`${unassignedFlights} aktive spillere mangler flight.`)
  if (isTeamScoringFormat(pairings.scoring_format)) {
    const teamLabels = teamReadinessLabels[pairings.scoring_format]
    if (!teamLabels) throw new Error('Teamformat mangler klartekst for oppsett')
    const assignedTeams = new Set(draft.teams.flatMap((team) => team.memberIds))
    const unassignedTeams = pairings.active_entrants.filter((entrant) => !assignedTeams.has(entrant.player_id)).length
    if (unassignedTeams > 0) unresolved.push(`${unassignedTeams} aktive spillere mangler ${teamLabels.assignment}.`)
    const incompleteTeams = draft.teams.filter((team) => team.memberIds.length !== 2).length
    if (incompleteTeams > 0) unresolved.push(`${incompleteTeams} ${teamLabels.size} har ikke nøyaktig to spillere.`)
  }
  return { blocking, unresolved }
}
