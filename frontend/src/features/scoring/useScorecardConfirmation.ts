import { useEffect } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../../api/client'
import { ApiHttpError } from '../../api/http'
import { scoringKeys, type ScoreOwner, type ScorecardSummary } from '../../api/scorecards'
import { tournamentKeys } from '../../api/tournaments'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import type { Round } from '../../api/types'
import { invalidateScorecard, invalidateScoreDependents } from './queries'
import { useAuth } from '../auth/authContext'

interface ConfirmationInput {
  round: Round
  tournamentId: string
  owner: ScoreOwner
  card: ScorecardSummary
  csrfToken: string | null
  onConfirmed: () => void
  onTerminal: () => void
}

type ConfirmationVariables = ConfirmationInput

export function useScorecardConfirmation(input: ConfirmationInput) {
  const queryClient = useQueryClient()
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const mutation = useMutation({
    mutationKey: ['scorecard-confirmation', input.round.id, input.owner.type, input.owner.id],
    mutationFn: async (variables: ConfirmationVariables) => {
      if (!variables.csrfToken) throw new Error('Økten er utløpt')
      if (!variables.card.complete || variables.card.confirmed) {
        throw new Error('Scorekortet må være komplett og ubekreftet')
      }
      return api.confirmScorecard(variables.round.id, variables.owner, variables.csrfToken)
    },
    onSuccess: async (card, variables) => {
      queryClient.setQueryData(scoringKeys.scorecard(userId, variables.round.id, variables.owner), card)
      await invalidateScorecard(queryClient, userId, variables.round.id, variables.owner)
      queryClient.setQueryData(scoringKeys.scorecard(userId, variables.round.id, variables.owner), card)
      await invalidateScoreDependents(queryClient, userId, variables.round.id, variables.tournamentId)
      variables.onConfirmed()
    },
    onError: async (error, variables) => {
      if (!(error instanceof ApiHttpError) || error.code !== 'round_not_editable') return
      variables.onTerminal()
      const [round, completion, card] = await Promise.all([
        api.round(variables.round.id),
        api.completionValidation(variables.round.id, variables.round.scoring_format),
        api.scorecard(variables.round.id, variables.owner),
      ])
      queryClient.setQueryData(tournamentKeys.round(userId, variables.round.id), round)
      queryClient.setQueryData(privateWorkspaceKeys.completion(userId, variables.round.id), completion)
      queryClient.setQueryData(scoringKeys.scorecard(userId, variables.round.id, variables.owner), card)
      await queryClient.invalidateQueries({ queryKey: tournamentKeys.rounds(userId, variables.tournamentId), exact: true })
    },
  })
  const reset = mutation.reset
  const scopeKey = `${input.round.id}:${input.owner.type}:${input.owner.id}`

  useEffect(() => reset(), [reset, scopeKey])

  let errorMessage: string | null = null
  let retryable = true
  if (mutation.error instanceof ApiHttpError && (mutation.error.status === 401 || mutation.error.status === 403)) {
    errorMessage = mutation.error.status === 401
      ? 'Økten er utløpt. Logg inn på nytt.'
      : 'Du har ikke tilgang til dette scorekortet.'
    retryable = false
  } else if (mutation.error instanceof ApiHttpError && mutation.error.code === 'round_not_editable') {
    errorMessage = 'Runden kan ikke lenger redigeres.'
    retryable = false
  } else if (mutation.error) {
    errorMessage = mutation.error.message
  }

  return {
    confirm: () => mutation.mutate({
      ...input,
      owner: input.owner.type === 'player'
        ? { type: 'player', id: input.owner.id }
        : { type: 'team', id: input.owner.id },
    }),
    confirming: mutation.isPending,
    errorMessage,
    retryable,
  }
}
