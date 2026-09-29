import type { QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../../api/auth'
import { ApiHttpError } from '../../../api/http'
import { tournamentKeys } from '../../../api/tournaments'
import type { Round, Tournament } from '../../../api/types'
import { privateWorkspaceKeys } from '../../../api/privateWorkspace'
import { scoringKeys, ownerEquals, type ScoreOwner, type ScoreAccess, type RoundCompletionValidation, type ScoringScorecard } from '../../../api/scorecards'
import type { FourBallCard } from '../../../api/fourBall'
import type { StablefordCard } from '../../../api/stableford'
export interface PreparedSelection { tournamentId: string; roundId: string; owner: ScoreOwner; holeNumber: number }
export function preparedKeys(userId: string, target: PreparedSelection) {
  return [authKeys.session, tournamentKeys.list(userId), tournamentKeys.rounds(userId, target.tournamentId),
    privateWorkspaceKeys.completion(userId, target.roundId), privateWorkspaceKeys.scoreAccess(userId, target.roundId),
    scoringKeys.scoring(userId, target.roundId, target.owner)]
}
export function isTerminal(error: unknown): boolean {
  return error instanceof ApiHttpError && ([401, 403, 404].includes(error.status) || error.code === 'round_not_editable')
}
// Read existing Query data only. A prepared target never retains a second card,
// visibility projection, permission response or server revision.
export function eligiblePrepared(client: QueryClient, session: AuthSession | null, target: PreparedSelection): boolean {
  if (!session || Date.parse(session.expires_at) <= Date.now()) return false
  const current = client.getQueryData<AuthSession | null>(authKeys.session)
  if (current?.user_id !== session.user_id || current.csrf_token !== session.csrf_token) return false
  const keys = preparedKeys(session.user_id, target)
  if (keys.some(key => isTerminal(client.getQueryState(key)?.error))) return false
  const tournaments = client.getQueryData<Tournament[]>(tournamentKeys.list(session.user_id))
  const round = client.getQueryData<Round[]>(tournamentKeys.rounds(session.user_id, target.tournamentId))?.find(r => r.id === target.roundId)
  const detail = client.getQueryData<Round>(tournamentKeys.round(session.user_id, target.roundId))
  const completion = client.getQueryData<RoundCompletionValidation>(privateWorkspaceKeys.completion(session.user_id, target.roundId))
  const access = client.getQueryData<ScoreAccess>(privateWorkspaceKeys.scoreAccess(session.user_id, target.roundId))
  const card = client.getQueryData<ScoringScorecard | FourBallCard | StablefordCard>(scoringKeys.scoring(session.user_id, target.roundId, target.owner))
  if (!tournaments?.some(t => t.id === target.tournamentId) || !round || round.tournament_id !== target.tournamentId
    || round.scoring_format === 'singles_match_play') return false
  if ([round, detail, completion].some(value => value && !['open', 'completed'].includes(value.status))) return false
  if (isTerminal(client.getQueryState(tournamentKeys.round(session.user_id, target.roundId))?.error)) return false
  return !!access?.writable_owners.some(owner => ownerEquals(owner, target.owner)) && card?.projection === 'scoring'
    && card.round_id === target.roundId && ownerEquals(card.owner, target.owner)
    && card.holes.some(hole => hole.hole_number === target.holeNumber)
}
export function freshPreparation(client: QueryClient, session: AuthSession | null, target: PreparedSelection, after: number): boolean {
  return !!session && eligiblePrepared(client, session, target) && preparedKeys(session.user_id, target).every(key => {
    const state = client.getQueryState(key)
    return state?.status === 'success' && state.fetchStatus === 'idle' && (key[0] === 'auth' || state.dataUpdatedAt > after)
  })
}
