import type {
  LegacyConversionInput,
  PairingFlightInput,
  PairingReplacement,
  PairingTeamInput,
  RoundPairings,
} from '../../../api/pairings'
import { isTeamScoringFormat } from '../../../api/scoringFormats'
import type { PairingDraft, PairingDraftGroup } from './draft'

export function serializedTeeTime(group: PairingDraftGroup): string | null {
  if (!group.teeTimeEdited) return group.sourceTeeTime
  return group.teeTime === '' ? null : `${group.teeTime}:00`
}

function serializedMembers(group: PairingDraftGroup): { player_id: string }[] {
  return group.memberIds.map((player_id) => ({ player_id }))
}

export function replacementFromDraft(
  draft: PairingDraft,
  format: RoundPairings['scoring_format'],
): PairingReplacement {
  const teams: PairingTeamInput[] = isTeamScoringFormat(format) ? draft.teams.map((team) => ({
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
