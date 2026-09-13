import { useEffect, useRef } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../../api/client'
import { ApiHttpError } from '../../api/http'
import { scoringKeys, type ScoreOwner, type ScoringScorecard } from '../../api/scorecards'
import { scoreRequest } from '../../api/scorecards/timeout'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import type { Round } from '../../api/types'
import { invalidateScorecard, invalidateScoreDependents } from './queries'
import { useAuth } from '../auth/authContext'
import { useScoreQueue } from './offline/context'
import { queueDatabase } from './offline/database'
import { cardKey, REQUEST_MS } from './offline/model'

interface ConfirmationInput {
  round: Round
  tournamentId: string
  owner: ScoreOwner
  card: ScoringScorecard
  csrfToken: string | null
  onConfirmed: () => void
  onTerminal: () => void
}
export function useScorecardConfirmation(input: ConfirmationInput) {
  const client = useQueryClient()
  const { session } = useAuth()
  const queue = useScoreQueue()
  const userId = session?.user_id ?? ''
  const key = cardKey({ accountId: userId, roundId: input.round.id, owner: input.owner })
  const busy = useRef(false)
  const current = useRef(key)
  current.current = key
  const controller = useRef<AbortController | null>(null)
  useEffect(() => () => controller.current?.abort(), [])
  const blocked = !queue.online || queue.loading || queue.error !== null || queue.items.some(item => item.cardKey === key) || queue.refreshing.some(value => value.item.cardKey === key)
  const mutation = useMutation({ gcTime: 0, retry: false, networkMode: 'always',
    mutationKey: ['scorecard-confirmation', input.round.id, input.owner.type, input.owner.id],
    mutationFn: async () => {
      if (!input.csrfToken || !navigator.onLine || blocked || !queue.runtime.isCurrent()) throw new Error('Alle endringer må leveres og forbindelsen må være tilbake før du bekrefter.')
      const lease = crypto.randomUUID()
      const own = () => current.current === key && queue.runtime.isCurrent()
      const until = await queueDatabase.acquireConfirmation(key, lease)
      try {
        if (!own()) return
        const abort = new AbortController(); controller.current = abort
        await scoreRequest(async signal => {
          const fresh = await api.scorecardScoring(input.round.id, input.owner, signal)
          if (!own() || signal.aborted) return
          if (Date.now() + REQUEST_MS > until) throw new Error('Bekreftelsen tok for lang tid. Oppdater kortet og prøv igjen.')
          client.setQueryData(scoringKeys.scoring(userId, input.round.id, input.owner), fresh)
          if (fresh.confirmed) { input.onConfirmed(); return }
          if (!fresh.complete) throw new Error('Scorekortet må være komplett før du bekrefter.')
          const card = await api.confirmScorecard(input.round.id, input.owner, input.csrfToken ?? '', signal)
          if (!own() || signal.aborted) return
          await invalidateScorecard(client, userId, input.round.id, input.owner)
          if (!own()) return
          client.setQueryData(scoringKeys.scoring(userId, input.round.id, input.owner), card)
          void invalidateScoreDependents(client, userId, input.round.id, input.tournamentId)
          input.onConfirmed()
        }, abort.signal)
      } finally { await queueDatabase.releaseConfirmation(key, lease) }
    },
    onError: error => {
      if (!queue.runtime.isCurrent() || current.current !== key) return
      if (error instanceof ApiHttpError && (error.status === 401 || error.status === 403 || error.code === 'round_not_editable')) {
        input.onTerminal()
        client.removeQueries({ queryKey: scoringKeys.scoring(userId, input.round.id, input.owner), exact: true })
        void client.invalidateQueries({ queryKey: privateWorkspaceKeys.scoreAccess(userId, input.round.id), exact: true })
        void client.invalidateQueries({ queryKey: privateWorkspaceKeys.completion(userId, input.round.id), exact: true })
      }
    },
    onSettled: () => { busy.current = false },
  })
  const reset = mutation.reset
  useEffect(() => { reset(); busy.current = false }, [reset, key])
  return {
    confirm: () => { if (!busy.current && !blocked) { busy.current = true; mutation.mutate() } },
    confirming: mutation.isPending,
    blocked,
    errorMessage: mutation.error ? mutation.error instanceof ApiHttpError ? 'Scorekortet kunne ikke bekreftes. Oppdater tilgang og rundestatus.'
      : mutation.error.name === 'AbortError' ? 'Bekreftelsen tok for lang tid. Oppdater scorekortet før du prøver igjen.' : mutation.error.message : null,
    retryable: !(mutation.error instanceof ApiHttpError && (mutation.error.status === 401 || mutation.error.status === 403 || mutation.error.code === 'round_not_editable')),
  }
}
