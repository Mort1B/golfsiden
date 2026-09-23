import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useMemo } from 'react'
import { stablefordApi, type StablefordCard } from '../../api/stableford'
import { fourBallApi, type FourBallCard } from '../../api/fourBall'
import { api } from '../../api/client'
import { ownerEquals, scoringKeys, type ScorecardSummary } from '../../api/scorecards'
import { ApiHttpError } from '../../api/http'
import { tournamentKeys } from '../../api/tournaments'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import { useAuth } from '../auth/authContext'
import { useTournamentLive } from '../live/useTournamentLive'
import { useScoreResume } from './scoreResumeContext'
import { resumeTournamentLive } from '../../api/tournamentLive'
import { parseHoleNumber, parseScoreView, preferredScoreRound, scoreableRounds,
  selectedOwner, adjacentWritableOwners, writableOwnerProgress, canonicalVisibleHole } from './selection'

export function useScoreWorkspaceData(searchParams: URLSearchParams, resume: boolean) {
  const { selection, remember } = useScoreResume()
  const fresh = { staleTime: resume ? 0 : 20_000, refetchOnMount: resume ? 'always' as const : true }
  const queryClient = useQueryClient()
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const tournamentsQuery = useQuery({ queryKey: tournamentKeys.list(userId), queryFn: api.tournaments, ...fresh })
  const tournaments = tournamentsQuery.data ?? []
  const tournament = tournaments.find((item) => item.id === (searchParams.get('tournament') ?? (resume ? selection?.tournamentId : null)))
    ?? tournaments.find((item) => item.status === 'active')
    ?? tournaments[0]
  const connectionLost = useTournamentLive(tournament?.id ?? '')
  const roundsQuery = useQuery({
    ...fresh,
    queryKey: tournamentKeys.rounds(userId, tournament?.id ?? ''),
    queryFn: () => api.rounds(tournament?.id ?? ''),
    enabled: tournament !== undefined,
  })
  const eligibleRounds = scoreableRounds(roundsQuery.data ?? [])
  const round = eligibleRounds.find((item) => item.id === (searchParams.get('round') ?? (resume && tournament?.id === selection?.tournamentId ? selection?.roundId : null)))
    ?? preferredScoreRound(eligibleRounds)
  const completionQuery = useQuery({
    ...fresh,
    queryKey: privateWorkspaceKeys.completion(userId, round?.id ?? ''),
    queryFn: () => api.completionValidation(round?.id ?? '', round?.scoring_format ?? 'individual_stroke_play'),
    enabled: round !== undefined && round.scoring_format !== 'singles_match_play',
  })
  const accessQuery = useQuery({
    ...fresh,
    queryKey: privateWorkspaceKeys.scoreAccess(userId, round?.id ?? ''),
    queryFn: () => api.scoreAccess(round?.id ?? ''),
    enabled: round !== undefined && round.scoring_format !== 'singles_match_play',
    retry: false,
  })
  const progressOwners = completionQuery.data?.owners ?? []
  const writableOwners = accessQuery.data?.writable_owners ?? []
  const completionDenied = completionQuery.error instanceof ApiHttpError
    && [401, 403, 404].includes(completionQuery.error.status)
  const accessDenied = accessQuery.error instanceof ApiHttpError
    && [401, 403, 404].includes(accessQuery.error.status)
  // Completion is visibility-projected and cleared on disconnect. The exact
  // authorized writable card is not: keep its mounted input/intent, not a copy
  // of the cleared names or progress, until fresh completion metadata arrives.
  const recoveringOwner = !resume && completionQuery.data === undefined && !completionDenied
    && !accessDenied && searchParams.get('tournament') === tournament?.id
    && searchParams.get('round') === round?.id && (round?.status === 'open' || round?.status === 'completed')
    ? writableOwners.find(item => item.type === searchParams.get('owner_type') && item.id === searchParams.get('owner'))
    : undefined
  const requestedOwner = selectedOwner(
    progressOwners,
    searchParams.get('owner_type') ?? (resume && tournament?.id === selection?.tournamentId && round?.id === selection?.roundId ? selection?.owner.type ?? null : null),
    searchParams.get('owner') ?? (resume && tournament?.id === selection?.tournamentId && round?.id === selection?.roundId ? selection?.owner.id ?? null : null),
    writableOwners,
  )
  const owner = useMemo(() => requestedOwner
    ?? (recoveringOwner ? { owner: recoveringOwner, owner_name: 'Valgt scorekort' } : undefined),
  [requestedOwner, recoveringOwner])
  const effectiveRoundStatus = completionQuery.data?.status ?? round?.status
  const canWrite = owner !== undefined
    && (effectiveRoundStatus === 'open' || effectiveRoundStatus === 'completed')
    && writableOwners.some((writable) => ownerEquals(writable, owner.owner))
  const queryOwner = owner?.owner ?? { type: 'player' as const, id: '' }
  const cardQuery = useQuery<ScorecardSummary | FourBallCard | StablefordCard>({
    ...fresh,
    queryKey: canWrite
      ? scoringKeys.scoring(userId, round?.id ?? '', queryOwner)
      : scoringKeys.read(userId, round?.id ?? '', queryOwner),
    queryFn: () => round?.scoring_format === 'individual_stableford'
      ? canWrite ? stablefordApi.scoring(round.id, queryOwner.id) : stablefordApi.read(round.id, queryOwner.id)
      : round?.scoring_format === 'four_ball_stroke_play'
      ? canWrite ? fourBallApi.scoring(round.id, queryOwner.id) : fourBallApi.read(round.id, queryOwner.id)
      : canWrite
      ? api.scorecardScoring(round?.id ?? '', queryOwner)
      : api.scorecardRead(round?.id ?? '', queryOwner),
    enabled: round !== undefined && owner !== undefined && accessQuery.data !== undefined,
    retry: false,
  })
  const terminalScoringError = canWrite && cardQuery.error instanceof ApiHttpError
    && (cardQuery.error.status === 401 || cardQuery.error.status === 403 || cardQuery.error.status === 404
      || cardQuery.error.code === 'round_not_editable')
  const retainingScorer = recoveringOwner !== undefined && cardQuery.data?.projection === 'scoring'
    && !terminalScoringError && cardQuery.data.holes.some(item => item.hole_number === parseHoleNumber(searchParams.get('hole')))
  const refetchAccess = accessQuery.refetch
  const refetchCompletion = completionQuery.refetch
  const refetchRounds = roundsQuery.refetch
  useEffect(() => {
    if (!terminalScoringError || !round || !owner) return
    queryClient.removeQueries({ queryKey: scoringKeys.scoring(userId, round.id, owner.owner), exact: true })
    void Promise.all([refetchCompletion(), refetchAccess(), refetchRounds()])
  }, [owner, queryClient, refetchAccess, refetchCompletion, refetchRounds, round, terminalScoringError, userId])
  const resumeCard = resume && cardQuery.data?.projection === 'scoring' ? cardQuery.data : null
  const missingHole = resumeCard?.holes.filter((item) => 'players' in item ? item.gross === null : item.score === null)
    .sort((a, b) => a.hole_number - b.hole_number)[0]
  const view = resumeCard && resumeCard.holes.length > 0 && !missingHole ? 'summary' : parseScoreView(searchParams.get('view'))
  const requestedHole = missingHole?.hole_number ?? parseHoleNumber(searchParams.get('hole'))
  const visibleHole = canonicalVisibleHole(cardQuery.data?.holes.map((item) => item.hole_number) ?? [], requestedHole)
  const hole = cardQuery.data?.holes.find((item) => item.hole_number === visibleHole)

  const prefetchOwner = useCallback((nextOwner: NonNullable<typeof owner>['owner']) => {
    if (!round) return
    void queryClient.prefetchQuery<ScorecardSummary | FourBallCard | StablefordCard>({
      queryKey: scoringKeys.scoring(userId, round.id, nextOwner),
      queryFn: () => round.scoring_format === 'individual_stableford' ? stablefordApi.scoring(round.id, nextOwner.id) : round.scoring_format === 'four_ball_stroke_play' ? fourBallApi.scoring(round.id, nextOwner.id) : api.scorecardScoring(round.id, nextOwner),
    })
  }, [queryClient, round, userId])

  useEffect(() => {
    if (!owner) return
    const writableCards = writableOwnerProgress(
      completionQuery.data?.owners ?? [],
      accessQuery.data?.writable_owners ?? [],
    )
    for (const neighbor of adjacentWritableOwners(writableCards, owner.owner)) {
      prefetchOwner(neighbor)
    }
  }, [accessQuery.data?.writable_owners, completionQuery.data?.owners, owner, prefetchOwner])

  useEffect(() => {
    if (!resume && tournament && round && owner && cardQuery.isSuccess) {
      remember({ tournamentId: tournament.id, roundId: round.id, owner: owner.owner })
    }
  }, [resume, tournament, round, owner, cardQuery.isSuccess, remember])

  return { tournamentsQuery, tournaments, tournament, roundsQuery, eligibleRounds, round,
    completionQuery, accessQuery, progressOwners, writableOwners, owner, effectiveRoundStatus,
    canWrite, cardQuery, terminalScoringError, view, hole, prefetchOwner, retainingScorer,
    deniedError: completionDenied ? completionQuery.error : accessDenied ? accessQuery.error : null,
    connectionLost, retryLive: () => resumeTournamentLive(userId, tournament?.id ?? '') }
}
