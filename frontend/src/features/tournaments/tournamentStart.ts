import { ApiHttpError } from '../../api/http'
import type { Round, Tournament, TournamentPlayerRoster } from '../../api/types'

export type ReadinessState = 'pending' | 'error' | 'ready' | 'missing'

export interface TournamentStartReadiness {
  roundPlan: ReadinessState
  draftRounds: ReadinessState
  activeEntrant: ReadinessState
  canStart: boolean
  numberedRoundCount: number
}

interface StartReadinessInput {
  tournament: Tournament
  rounds: Round[] | undefined
  roundsPending: boolean
  roundsError: Error | null
  roster: TournamentPlayerRoster | undefined
  rosterPending: boolean
  rosterError: Error | null
}

export function tournamentStartReadiness(input: StartReadinessInput): TournamentStartReadiness {
  const roundReadState: ReadinessState = input.roundsPending
    ? 'pending'
    : input.roundsError
      ? 'error'
      : 'missing'
  const rosterReadState: ReadinessState = input.rosterPending
    ? 'pending'
    : input.rosterError
      ? 'error'
      : 'missing'
  const expectedNumbers = new Set(
    Array.from({ length: input.tournament.number_of_rounds }, (_, index) => index + 1),
  )
  const numberedRounds = input.rounds?.filter((round) => (
    round.tournament_id === input.tournament.id && expectedNumbers.has(round.round_number)
  )) ?? []
  const uniqueRoundNumbers = new Set(numberedRounds.map((round) => round.round_number))
  const completeRoundPlan = input.rounds !== undefined
    && input.rounds.length === input.tournament.number_of_rounds
    && uniqueRoundNumbers.size === input.tournament.number_of_rounds
  const allRoundsDraft = input.rounds !== undefined
    && input.rounds.length > 0
    && input.rounds.every((round) => round.status === 'draft')
  const hasActiveEntrant = input.roster?.players.some((player) => (
    player.status === 'active' && player.player_active
  )) ?? false
  const roundPlan = input.rounds === undefined
    ? roundReadState
    : completeRoundPlan ? 'ready' : 'missing'
  const draftRounds = input.rounds === undefined
    ? roundReadState
    : allRoundsDraft ? 'ready' : 'missing'
  const activeEntrant = input.roster === undefined
    ? rosterReadState
    : hasActiveEntrant ? 'ready' : 'missing'

  return {
    roundPlan,
    draftRounds,
    activeEntrant,
    canStart: input.tournament.status === 'draft'
      && roundPlan === 'ready'
      && draftRounds === 'ready'
      && activeEntrant === 'ready',
    numberedRoundCount: uniqueRoundNumbers.size,
  }
}

export interface TournamentStartFailure {
  message: string
  refresh: 'none' | 'tournament' | 'all'
}

export function tournamentStartFailure(error: Error | null): TournamentStartFailure | null {
  if (!error) return null
  if (error instanceof ApiHttpError) {
    if (error.code === 'tournament_start_stale') {
      return {
        message: 'Turneringen ble endret et annet sted. Vi henter oppdatert status; kontroller den og prøv igjen.',
        refresh: 'tournament',
      }
    }
    if (error.code === 'tournament_start_not_ready') {
      return {
        message: 'Serveren fant at turneringen ikke er klar. Vi henter runder og deltakere på nytt; kontroller listen før du prøver igjen.',
        refresh: 'all',
      }
    }
    if (error.code === 'tournament_start_invalid_state') {
      return {
        message: 'Turneringen kan ikke startes fra gjeldende status. Oppdatert turneringsstatus hentes.',
        refresh: 'tournament',
      }
    }
  }
  return { message: error.message, refresh: 'none' }
}
