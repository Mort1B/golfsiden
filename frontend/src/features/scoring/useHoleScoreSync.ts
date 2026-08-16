import { useCallback, useEffect, useMemo, useRef, useSyncExternalStore } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { api } from '../../api/client'
import { scoringKeys, type ScoreOwner, type ScorecardSummary } from '../../api/scorecards'
import { tournamentKeys } from '../../api/tournaments'
import type { Round } from '../../api/types'
import { invalidateScorecard, invalidateScoreDependents } from './queries'
import { ScoreCoordinator, hasUnresolvedIntent } from './scoreCoordinator'

interface HoleScoreSyncInput {
  round: Round
  tournamentId: string
  owner: ScoreOwner
  holeId: string
  serverValue: number | null
  csrfToken: string | null
  onVerified: (card: ScorecardSummary) => void
  onTerminal: () => void
}

function holeValue(card: ScorecardSummary, holeId: string): number | null {
  return card.holes.find((hole) => hole.hole_id === holeId)?.score?.gross_strokes ?? null
}

export function useHoleScoreSync(input: HoleScoreSyncInput) {
  const queryClient = useQueryClient()
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
      await api.saveScore(input.round.id, input.holeId, owner, value, input.csrfToken)
    },
    verify: async () => {
      await invalidateScorecard(queryClient, input.round.id, owner)
      const card = await api.scorecard(input.round.id, owner)
      queryClient.setQueryData(scoringKeys.scorecard(input.round.id, owner), card)
      await invalidateScoreDependents(queryClient, input.round.id, input.tournamentId)
      verifiedRef.current(card)
      return holeValue(card, input.holeId)
    },
    terminalRefresh: async () => {
      terminalRef.current()
      const [round, completion, card] = await Promise.all([
        api.round(input.round.id),
        api.completionValidation(input.round.id, input.round.scoring_format),
        api.scorecard(input.round.id, owner),
      ])
      queryClient.setQueryData(['round', input.round.id], round)
      queryClient.setQueryData(scoringKeys.completion(input.round.id), completion)
      queryClient.setQueryData(scoringKeys.scorecard(input.round.id, owner), card)
      await queryClient.invalidateQueries({
        queryKey: tournamentKeys.rounds(input.tournamentId),
        exact: true,
      })
      await invalidateScoreDependents(queryClient, input.round.id, input.tournamentId)
      verifiedRef.current(card)
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
