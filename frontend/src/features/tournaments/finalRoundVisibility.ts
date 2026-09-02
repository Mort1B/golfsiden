import { ApiHttpError } from '../../api/http'
import type { Round, Tournament } from '../../api/types'

export function applicableFinalRound(tournament: Tournament, rounds: Round[]): Round | null {
  return rounds.find((round) => round.tournament_id === tournament.id
    && round.round_number === tournament.number_of_rounds
    && round.number_of_holes === 18
    && round.course_id !== null
    && round.tee_id !== null) ?? null
}

export function finalRoundVisibilityFailure(error: Error | null): {
  message: string
  refetch: boolean
} | null {
  if (error === null) return null
  if (error instanceof ApiHttpError && error.code === 'final_round_visibility_stale') {
    return {
      message: 'Synligheten ble endret et annet sted. Serverstatusen er hentet på nytt.',
      refetch: true,
    }
  }
  return { message: error.message, refetch: false }
}
