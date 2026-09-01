import { ApiHttpError } from '../../api/http'
import type { Round, TournamentStatus } from '../../api/types'

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
        message: 'Valget er permanent låst fordi turneringen er startet, eller en runde har vært åpnet og konfigurasjonen er fryst. Oppdaterte turneringsfakta er hentet.',
        refetch: true,
      }
    }
  }
  return { message: error.message, refetch: false }
}

export function countedRoundsAreEditable(
  tournamentStatus: TournamentStatus,
  rounds: Round[] | undefined,
): boolean {
  return tournamentStatus === 'draft'
    && rounds !== undefined
    && rounds.every((round) => round.status === 'draft')
}
