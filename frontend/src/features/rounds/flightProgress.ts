import type { RoundPairings } from '../../api/pairings'
import type { OwnerCompletionProgress, RoundCompletionValidation } from '../../api/scorecards'
import { isTeamScoringFormat } from '../../api/scoringFormats'

export interface FlightProgress {
  id: string
  name: string
  owners: OwnerCompletionProgress[]
  scored: number
  required: number
  completed: number | null
  confirmed: number | null
}

// Pairings freeze on opening. Join preserved owner IDs, never current team/roster
// status or scheduling metadata. Every required card must resolve exactly once.
export function flightProgress(pairings: RoundPairings, progress: RoundCompletionValidation): FlightProgress[] {
  if (pairings.round_id !== progress.round_id || pairings.status !== progress.status || pairings.status === 'draft') {
    throw new Error('Runden er endret. Oppdater fremdriften og rundeoppsettet.')
  }
  const teamFormat = isTeamScoringFormat(pairings.scoring_format)
  const assignments = new Map<string, OwnerCompletionProgress[]>()
  for (const flight of pairings.flights) assignments.set(flight.id, [])
  for (const owner of progress.owners) {
    if (owner.owner.type !== (teamFormat ? 'team' : 'player')) throw new Error('Scorekortene samsvarer ikke med spilleformen.')
    const team = pairings.teams.find((item) => item.id === owner.owner.id)
    const members = teamFormat ? team?.members.map((member) => member.player_id) : [owner.owner.id]
    if (!members?.length) throw new Error('Scorekortet mangler et bevart lagoppsett. Oppdater runden.')
    const flights = pairings.flights.filter((flight) => members.every((id) => flight.members.some((member) => member.player_id === id)))
    const match = flights.length === 1 ? flights[0] : undefined
    if (!match) throw new Error('Fremdrift kan ikke grupperes: scorekortet mangler entydig, bevart flighttilknytning. Eldre runder kan mangle flightoppsett.')
    assignments.get(match.id)?.push(owner)
  }
  const visibleOnly = progress.visibility.mode === 'front_nine'
  return pairings.flights.map((flight) => {
    const owners = assignments.get(flight.id) ?? []
    return {
      id: flight.id, name: flight.name, owners,
      scored: owners.reduce((sum, owner) => sum + owner.holes_scored, 0),
      required: owners.reduce((sum, owner) => sum + owner.required_holes, 0),
      completed: visibleOnly ? null : owners.filter((owner) => owner.complete === true).length,
      confirmed: visibleOnly ? null : owners.filter((owner) => owner.confirmed === true).length,
    }
  })
}
