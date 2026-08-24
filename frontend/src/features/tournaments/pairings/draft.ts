import type {
  PairingGroup,
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

function sameMembers(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((member) => right.includes(member))
}

export function scheduleFlightOptions(team: PairingDraftGroup, flights: PairingDraftGroup[]): PairingDraftGroup[] {
  return flights.filter((flight) => sameMembers(team.memberIds, flight.memberIds))
}

export function pairingsFingerprint(pairings: RoundPairings): string {
  return JSON.stringify(pairings)
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
