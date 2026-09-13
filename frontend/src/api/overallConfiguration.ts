import { invalidData } from './decoder'
import { contributesToOverall } from './scoringFormats'
import type { Round, Tournament } from './types'
export function validateOverallConfiguration(tournament: Tournament, rounds: Round[]): void {
  const eligible = rounds.filter(r => contributesToOverall(r.scoring_format))
  if (eligible.length === 0 ? tournament.counted_rounds !== null || tournament.mandatory_round_id !== null
    : tournament.counted_rounds === null || tournament.counted_rounds < 1 || tournament.counted_rounds > eligible.length
      || tournament.mandatory_round_id !== null && !eligible.some(r => r.id === tournament.mandatory_round_id)) invalidData('turneringsdata', 'overall configuration')
}
