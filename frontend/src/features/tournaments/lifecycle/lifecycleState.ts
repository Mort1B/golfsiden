import type { PairingValidation, RoundTransition, ReadinessIssueCode } from '../../../api/roundLifecycle'
import type { RoundCompletionValidation } from '../../../api/scorecards'
import type { Round } from '../../../api/types'
import { ApiHttpError } from '../../../api/http'
import type { ManagementSectionId } from '../managementWorkspace'
import type { QueryClient } from '@tanstack/react-query'

export const transitionLabels: Record<RoundTransition, string> = {
  open: 'Åpne runden', complete: 'Fullfør runden', lock: 'Lås runden',
}

export const transitionExplanations: Record<RoundTransition, string> = {
  open: 'Åpning fryser bane, utslagssted og spillegrupper og lagrer rundens handicapgrunnlag. Oppsettet kan ikke endres etterpå.',
  complete: 'Synlig score fra den åpne runden kan allerede inngå foreløpig i sammenlagtresultatet. Fullføring gjør runden til et fullført resultat i beregningen av tellende runder og kvalifisering. Scorekort kan fortsatt korrigeres, men må da bekreftes på nytt før låsing.',
  lock: 'Låsing stenger for ordinære scoreendringer. Det finnes ingen funksjon for å låse opp runden. Kontroller scorekortene før du fortsetter.',
}

export function roundAction(status: Round['status']): RoundTransition | null {
  return status === 'draft' ? 'open' : status === 'open' ? 'complete' : status === 'completed' ? 'lock' : null
}

export function lifecycleReady(round: Round, opening?: PairingValidation, completion?: RoundCompletionValidation): boolean {
  if (round.status === 'draft') return opening?.round_id === round.id && opening.ready
  if (completion?.round_id !== round.id || completion.status !== round.status || completion.visibility.mode !== 'full') return false
  return round.status === 'open' ? completion.ready_to_complete === true
    : round.status === 'completed' && completion.ready_to_lock === true
}

export function roundManagementUrl(tournamentId: string, roundId: string, section: ManagementSectionId = 'lifecycle'): string {
  return `/manage/tournaments/${tournamentId}?${new URLSearchParams({ round: roundId })}#${section}`
}

export function readinessDestination(code: ReadinessIssueCode): ManagementSectionId {
  if (code === 'tournament_not_openable' || code === 'round_not_draft') return 'lifecycle'
  if (code === 'no_active_entrants') return 'invitations'
  if (['missing_course', 'missing_tee', 'mismatched_course_tee', 'missing_handicap_ratings',
    'invalid_hole_count', 'invalid_hole_numbers', 'invalid_stroke_indexes'].includes(code)) return 'courses'
  return 'pairings'
}

export function lifecycleFailure(error: unknown): string {
  if (error instanceof ApiHttpError) {
    if (error.status === 401) return 'Økten er utløpt. Logg inn på nytt.'
    if (error.status === 403) return 'Du har ikke lenger administratortilgang til denne turneringen.'
    if (error.status === 404) return 'Runden finnes ikke lenger.'
    if (error.status === 409) return 'Runden kunne ikke endres. Status og krav kontrolleres på nytt; se hva som gjenstår nedenfor.'
  }
  return 'Svaret kunne ikke bekreftes. Kontroller oppdatert rundestatus før du prøver igjen.'
}

export function isLifecycleTarget(key: readonly unknown[], userId: string, tournamentId: string, roundId: string): boolean {
  if (key[0] !== 'private-workspace' || key[1] !== userId) return false
  if (key[2] === 'rounds') return key[3] === roundId
  if (key[2] === 'tournaments') return key[3] === tournamentId || key[3] === 'mine' || key[3] === 'list'
  return key[2] === 'leaderboards'
    && ((key[3] === 'round' && key[4] === roundId) || (key[3] === 'tournament' && key[4] === tournamentId))
}

export async function reconcileLifecycleQueries(client: QueryClient, userId: string, tournamentId: string, roundId: string): Promise<void> {
  const filters = { predicate: (query: { queryKey: readonly unknown[] }) => isLifecycleTarget(query.queryKey, userId, tournamentId, roundId) }
  // SSE may replace this refetch with a newer one. Cancellation is not a read failure;
  // inspect the current queries, and let fetching/authority gates hold pending actions.
  // Replace reads started before the mutation outcome, so they cannot restore old readiness.
  await client.invalidateQueries(filters)
  const failed = client.getQueryCache().findAll(filters).some((query) => query.isActive()
    && (query.state.status === 'error' || query.state.fetchStatus === 'paused'
      || (query.state.isInvalidated && query.state.fetchStatus === 'idle')))
  if (failed) throw new Error('Rundens data kunne ikke oppdateres.')
}
