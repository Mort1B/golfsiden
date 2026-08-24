import { ApiHttpError } from '../../api/http'
import type { Round } from '../../api/types'

export interface CountedRoundsFailure {
  message: string
  refetch: boolean
}

export function countedRoundsFailure(error: Error | null): CountedRoundsFailure | null {
  if (!error) return null
  if (error instanceof ApiHttpError) {
    if (error.code === 'tournament_configuration_stale') {
      return {
        message: 'Turneringen ble endret et annet sted. Oppdaterte turneringsfakta er hentet; kontroller valget og prøv igjen.',
        refetch: true,
      }
    }
    if (error.code === 'tournament_configuration_locked') {
      return {
        message: 'Valget er permanent låst fordi en runde har vært åpnet. Oppdaterte turneringsfakta er hentet.',
        refetch: true,
      }
    }
  }
  return { message: error.message, refetch: false }
}

export function countedRoundsAreEditable(
  rounds: Round[] | undefined,
): boolean {
  return rounds !== undefined
    && rounds.every((round) => round.status === 'draft')
}
