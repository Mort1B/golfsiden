import type {
  LegacyConversionInput,
  PairingFlightInput,
  PairingGroup,
  PairingReplacement,
  PairingTeamInput,
  RoundPairings,
} from '../../../api/pairings'

export type DraftGroupKind = 'team' | 'flight'

interface DraftSchedule {
  startingHole: string
  teeTime: string
  sourceTeeTime: string | null
  teeTimeEdited: boolean
}

export interface PairingDraftGroup {
  id: string
  name: string
  memberIds: string[]
  startingHole: string
  teeTime: string
  sourceTeeTime: string | null
  teeTimeEdited: boolean
  scheduleTransferBackup: DraftSchedule | null
  scheduleFlightId: string | null
  requiresScheduleTransfer: boolean
}

export interface PairingDraft {
  sourceUpdatedAt: string
  sourceFingerprint: string
  teams: PairingDraftGroup[]
  flights: PairingDraftGroup[]
}

export interface DraftValidation {
  blocking: string[]
  unresolved: string[]
}

function editableTime(value: string | null): string {
  return value?.slice(0, 5) ?? ''
}

function draftGroup(group: PairingGroup, kind: DraftGroupKind): PairingDraftGroup {
  return {
    id: group.id,
    name: group.name,
    memberIds: group.members.map((member) => member.player_id),
    startingHole: String(group.starting_hole ?? ''),
    teeTime: editableTime(group.tee_time),
    sourceTeeTime: group.tee_time,
    teeTimeEdited: false,
    scheduleTransferBackup: null,
    scheduleFlightId: null,
    requiresScheduleTransfer: kind === 'team' && (group.starting_hole !== null || group.tee_time !== null),
  }
}

export function draftFromPairings(pairings: RoundPairings): PairingDraft {
  return {
    sourceUpdatedAt: pairings.updated_at,
    sourceFingerprint: pairingsFingerprint(pairings),
    teams: pairings.teams.map((group) => draftGroup(group, 'team')),
    flights: pairings.flights.map((group) => draftGroup(group, 'flight')),
  }
}

export function newDraftGroup(
  kind: DraftGroupKind,
  index: number,
  createId: () => string = () => crypto.randomUUID(),
): PairingDraftGroup {
  return {
    id: createId(),
    name: kind === 'team' ? `Lag ${index}` : `Flight ${index}`,
    memberIds: [],
    startingHole: '',
    teeTime: '',
    sourceTeeTime: null,
    teeTimeEdited: false,
    scheduleTransferBackup: null,
    scheduleFlightId: null,
    requiresScheduleTransfer: false,
  }
}

export function groupsFor(draft: PairingDraft, kind: DraftGroupKind): PairingDraftGroup[] {
  return kind === 'team' ? draft.teams : draft.flights
}

export function replaceGroups(
  draft: PairingDraft,
  kind: DraftGroupKind,
  groups: PairingDraftGroup[],
): PairingDraft {
  return kind === 'team' ? { ...draft, teams: groups } : { ...draft, flights: groups }
}

export function assignEntrant(
  draft: PairingDraft,
  kind: DraftGroupKind,
  playerId: string,
  groupId: string | null,
): PairingDraft {
  const groups = groupsFor(draft, kind).map((group) => ({
    ...group,
    memberIds: group.memberIds.filter((id) => id !== playerId),
  }))
  if (groupId !== null) {
    const index = groups.findIndex((group) => group.id === groupId)
    const group = groups[index]
    if (group) groups[index] = { ...group, memberIds: [...group.memberIds, playerId] }
  }
  return replaceGroups(draft, kind, groups)
}

export function moveMember(
  draft: PairingDraft,
  kind: DraftGroupKind,
  groupId: string,
  playerId: string,
  direction: -1 | 1,
): PairingDraft {
  const groups = groupsFor(draft, kind).map((group) => {
    if (group.id !== groupId) return group
    const from = group.memberIds.indexOf(playerId)
    const to = from + direction
    if (from < 0 || to < 0 || to >= group.memberIds.length) return group
    const memberIds = [...group.memberIds]
    const adjacent = memberIds[to]
    if (!adjacent) return group
    memberIds[to] = playerId
    memberIds[from] = adjacent
    return { ...group, memberIds }
  })
  return replaceGroups(draft, kind, groups)
}

function duplicateNames(groups: PairingDraftGroup[]): boolean {
  const names = groups.map((group) => group.name.trim().toLocaleLowerCase('nb-NO'))
  return new Set(names).size !== names.length
}

function duplicateMembers(groups: PairingDraftGroup[]): boolean {
  const ids = groups.flatMap((group) => group.memberIds)
  return new Set(ids).size !== ids.length
}

function sameMembers(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((member) => right.includes(member))
}

export function scheduleFlightOptions(team: PairingDraftGroup, flights: PairingDraftGroup[]): PairingDraftGroup[] {
  return flights.filter((flight) => sameMembers(team.memberIds, flight.memberIds))
}

export function pairingsFingerprint(pairings: RoundPairings): string {
  return JSON.stringify(pairings)
}

function serializedTeeTime(group: PairingDraftGroup): string | null {
  if (!group.teeTimeEdited) return group.sourceTeeTime
  return group.teeTime === '' ? null : `${group.teeTime}:00`
}

export function selectScheduleFlight(
  draft: PairingDraft,
  teamId: string,
  flightId: string | null,
): PairingDraft {
  const team = draft.teams.find((group) => group.id === teamId)
  if (!team) return draft
  const restoredFlights = draft.flights.map((group) => {
    if (group.id !== team.scheduleFlightId || !group.scheduleTransferBackup) return group
    return { ...group, ...group.scheduleTransferBackup, scheduleTransferBackup: null }
  })
  const flight = flightId ? restoredFlights.find((group) => group.id === flightId) : undefined
  return {
    ...draft,
    teams: draft.teams.map((group) => group.id === teamId
      ? { ...group, scheduleFlightId: flight?.id ?? null }
      : group),
    flights: restoredFlights.map((group) => group.id === flight?.id
      ? {
          ...group,
          scheduleTransferBackup: {
            startingHole: group.startingHole,
            teeTime: group.teeTime,
            sourceTeeTime: group.sourceTeeTime,
            teeTimeEdited: group.teeTimeEdited,
          },
          startingHole: team.startingHole,
          teeTime: team.teeTime,
          sourceTeeTime: team.sourceTeeTime,
          teeTimeEdited: team.teeTimeEdited,
        }
      : group),
  }
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
    const options = scheduleFlightOptions(team, draft.flights)
    const selected = options.find((flight) => flight.id === team.scheduleFlightId)
    if (!selected || selected.startingHole !== team.startingHole ||
      serializedTeeTime(selected) !== serializedTeeTime(team)) {
      blocking.push(`Velg eksplisitt hvilken flight som overtar starttiden fra ${team.name}.`)
    }
  }
  const assignedTeams = new Set(draft.teams.flatMap((team) => team.memberIds))
  const assignedFlights = new Set(draft.flights.flatMap((flight) => flight.memberIds))
  const unassignedFlights = pairings.active_entrants.filter((entrant) => !assignedFlights.has(entrant.player_id)).length
  if (unassignedFlights > 0) unresolved.push(`${unassignedFlights} aktive spillere mangler flight.`)
  if (pairings.scoring_format === 'team_scramble') {
    const unassignedTeams = pairings.active_entrants.filter((entrant) => !assignedTeams.has(entrant.player_id)).length
    if (unassignedTeams > 0) unresolved.push(`${unassignedTeams} aktive spillere mangler lag.`)
    const incompleteTeams = draft.teams.filter((team) => team.memberIds.length !== 2).length
    if (incompleteTeams > 0) unresolved.push(`${incompleteTeams} lag har ikke nøyaktig to spillere.`)
  }
  return { blocking, unresolved }
}

function serializedMembers(group: PairingDraftGroup): { player_id: string }[] {
  return group.memberIds.map((player_id) => ({ player_id }))
}

export function replacementFromDraft(draft: PairingDraft, format: RoundPairings['scoring_format']): PairingReplacement {
  const teams: PairingTeamInput[] = format === 'team_scramble' ? draft.teams.map((team) => ({
    id: team.id,
    name: team.name,
    members: serializedMembers(team),
    schedule_flight_id: team.scheduleFlightId,
  })) : []
  const flights: PairingFlightInput[] = draft.flights.map((flight) => ({
    id: flight.id,
    name: flight.name,
    starting_hole: flight.startingHole === '' ? null : Number(flight.startingHole),
    tee_time: serializedTeeTime(flight),
    members: serializedMembers(flight),
  }))
  return { expected_round_updated_at: draft.sourceUpdatedAt, teams, flights, legacy_conversions: [] }
}

export function legacyConversionReplacement(
  pairings: RoundPairings,
  createId: () => string = () => crypto.randomUUID(),
): PairingReplacement {
  const mappings: LegacyConversionInput[] = []
  const preservedFlights: PairingFlightInput[] = pairings.flights.map((group) => ({
    id: group.id,
    name: group.name,
    starting_hole: group.starting_hole,
    tee_time: group.tee_time,
    members: group.members.map(({ player_id }) => ({ player_id })),
  }))
  const convertedFlights = pairings.legacy_individual_groups.map((group) => {
    const flightId = createId()
    mappings.push({ team_id: group.id, flight_id: flightId })
    return {
      id: flightId,
      name: group.name,
      starting_hole: group.starting_hole,
      tee_time: group.tee_time,
      members: group.members.map(({ player_id }) => ({ player_id })),
    }
  })
  return {
    expected_round_updated_at: pairings.updated_at,
    teams: [],
    flights: [...preservedFlights, ...convertedFlights],
    legacy_conversions: mappings,
  }
}
