import { useCallback, useEffect, useMemo, useRef, useSyncExternalStore } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { api } from '../../api/client'
import { scoringKeys, type ScoreOwner, type ScorecardSummary, type ScoringScorecard } from '../../api/scorecards'
import { tournamentKeys } from '../../api/tournaments'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import type { Round } from '../../api/types'
import { invalidateScorecard, invalidateScoreDependents } from './queries'
import { ScoreCoordinator, hasUnresolvedIntent } from './scoreCoordinator'
import { useAuth } from '../auth/authContext'
import { ApiHttpError } from '../../api/http'

interface HoleScoreSyncInput {
  round: Round
  tournamentId: string
  owner: ScoreOwner
  holeId: string
  serverValue: number | null
  csrfToken: string | null
  onVerified: (card: ScoringScorecard) => void
  onTerminal: () => void
}

function holeValue(card: ScorecardSummary, holeId: string): number | null {
  return card.holes.find((hole) => hole.hole_id === holeId)?.score?.gross_strokes ?? null
}

export function useHoleScoreSync(input: HoleScoreSyncInput) {
  const queryClient = useQueryClient()
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const verifiedRef = useRef(input.onVerified)
  const terminalRef = useRef(input.onTerminal)
  const initialValueRef = useRef(input.serverValue)
  initialValueRef.current = input.serverValue
  const owner = useMemo<ScoreOwner>(() => input.owner.type === 'player'
    ? { type: 'player', id: input.owner.id }
    : { type: 'team', id: input.owner.id }, [input.owner.id, input.owner.type])
  verifiedRef.current = input.onVerified
  terminalRef.current = input.onTerminal

  const coordinator = useMemo(() => new ScoreCoordinator({
    roundId: input.round.id,
    owner,
    holeId: input.holeId,
  }, initialValueRef.current, {
    save: async (value) => {
      if (!input.csrfToken) throw new Error('Økten er utløpt')
      try {
        await api.saveScore(input.round.id, input.holeId, owner, value, input.csrfToken)
      } catch (error) {
        if (error instanceof ApiHttpError && (error.status === 401 || error.status === 403)) {
          terminalRef.current()
          queryClient.removeQueries({ queryKey: scoringKeys.scoring(userId, input.round.id, owner), exact: true })
          await queryClient.invalidateQueries({ queryKey: privateWorkspaceKeys.scoreAccess(userId, input.round.id), exact: true })
        }
        throw error
      }
    },
    verify: async () => {
      await invalidateScorecard(queryClient, userId, input.round.id, owner)
      const card = await api.scorecardScoring(input.round.id, owner)
      queryClient.setQueryData(scoringKeys.scoring(userId, input.round.id, owner), card)
      await invalidateScoreDependents(queryClient, userId, input.round.id, input.tournamentId)
      verifiedRef.current(card)
      return holeValue(card, input.holeId)
    },
    terminalRefresh: async () => {
      terminalRef.current()
      const [round, completion] = await Promise.all([
        api.round(input.round.id),
        api.completionValidation(input.round.id, input.round.scoring_format),
      ])
      const card = completion.status === 'locked'
        ? await api.scorecardRead(input.round.id, owner)
        : await api.scorecardScoring(input.round.id, owner)
      queryClient.setQueryData(tournamentKeys.round(userId, input.round.id), round)
      queryClient.setQueryData(privateWorkspaceKeys.completion(userId, input.round.id), completion)
      if (card.projection === 'scoring') {
        queryClient.setQueryData(scoringKeys.scoring(userId, input.round.id, owner), card)
        verifiedRef.current(card)
      } else {
        queryClient.removeQueries({ queryKey: scoringKeys.scoring(userId, input.round.id, owner), exact: true })
        queryClient.setQueryData(scoringKeys.read(userId, input.round.id, owner), card)
      }
      await queryClient.invalidateQueries({
        queryKey: tournamentKeys.rounds(userId, input.tournamentId),
        exact: true,
      })
      await invalidateScoreDependents(queryClient, userId, input.round.id, input.tournamentId)
      return holeValue(card, input.holeId)
    },
  }), [
    input.holeId,
    input.round.id,
    input.round.scoring_format,
    input.csrfToken,
    input.tournamentId,
    owner,
    queryClient,
    userId,
  ])

  const subscribe = useCallback((listener: () => void) => coordinator.subscribe(listener), [coordinator])
  const getSnapshot = useCallback(() => coordinator.current(), [coordinator])
  const snapshot = useSyncExternalStore(subscribe, getSnapshot, getSnapshot)

  useEffect(() => {
    coordinator.acceptServerValue(input.serverValue)
  }, [coordinator, input.serverValue])
  return {
    snapshot,
    navigationLocked: hasUnresolvedIntent(snapshot),
    setScore: (value: number) => coordinator.setDesired(value),
    retry: () => coordinator.retry(),
    discard: () => coordinator.discard(),
  }
}
