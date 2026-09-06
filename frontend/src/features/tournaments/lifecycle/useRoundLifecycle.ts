import { useEffect, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../../../api/client'
import { roundLifecycleApi, roundLifecycleKeys, type RoundTransition } from '../../../api/roundLifecycle'
import { privateWorkspaceKeys } from '../../../api/privateWorkspace'
import { tournamentKeys } from '../../../api/tournaments'
import type { Round, Tournament } from '../../../api/types'
import { useAuth } from '../../auth/authContext'
import { lifecycleFailure, lifecycleReady, reconcileLifecycleQueries, roundAction } from './lifecycleState'

export function useRoundLifecycle(tournament: Tournament, selectedRound: Round, authorityRefreshing: boolean) {
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const client = useQueryClient()
  const alive = useRef(false)
  const submitting = useRef(false)
  const [busy, setBusy] = useState(false)
  const [reconcileFailed, setReconcileFailed] = useState(false)
  const [message, setMessage] = useState<{ kind: 'error' | 'success'; text: string } | null>(null)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const detail = useQuery({
    queryKey: tournamentKeys.round(userId, selectedRound.id),
    queryFn: () => api.round(selectedRound.id),
    staleTime: 0,
    refetchOnMount: 'always',
  })
  const round = detail.data ?? selectedRound
  const validRound = round.tournament_id === tournament.id && round.id === selectedRound.id
  const opening = useQuery({
    queryKey: roundLifecycleKeys.validation(userId, round.id),
    queryFn: () => roundLifecycleApi.validation(round.id),
    enabled: validRound && !!detail.data && round.status === 'draft',
    staleTime: 0,
    refetchOnMount: 'always',
  })
  const completion = useQuery({
    queryKey: privateWorkspaceKeys.completion(userId, round.id),
    queryFn: () => api.completionValidation(round.id, round.scoring_format),
    enabled: validRound && !!detail.data && (round.status === 'open' || round.status === 'completed'),
    staleTime: 0,
    refetchOnMount: 'always',
  })
  const readiness = round.status === 'draft' ? opening : completion
  const needsReadiness = round.status !== 'locked'
  const inconsistent = !validRound || round.status !== selectedRound.status
    || (round.status !== 'draft' && needsReadiness && completion.data !== undefined
      && (completion.data.status !== round.status || completion.data.visibility.mode !== 'full'))
  const loading = detail.isPending || (needsReadiness && readiness.isPending)
  const refreshing = detail.isFetching || (needsReadiness && readiness.isFetching)
  const error = detail.error ?? (needsReadiness ? readiness.error : null)
  const canAct = !!auth.session && !auth.loading && !auth.error && !loading && !refreshing && !error
    && !busy && !reconcileFailed && !inconsistent && !authorityRefreshing
    && (round.status !== 'draft' || tournament.status === 'active')
    && lifecycleReady(round, opening.data, completion.data)
  const mutation = useMutation({
    mutationFn: (action: RoundTransition) => {
      const csrf = auth.session?.csrf_token
      if (!csrf) throw new Error('Økten mangler.')
      return roundLifecycleApi.transition(round.id, tournament.id, action, csrf)
    },
    retry: false,
  })

  const reconcile = async () => {
    // Invalidate existing entries only: a late response must not repopulate a signed-out user's cache.
    await reconcileLifecycleQueries(client, userId, tournament.id, round.id)
  }
  const refresh = async () => {
    if (submitting.current) return
    submitting.current = true
    setBusy(true)
    try {
      await reconcile()
      if (alive.current) setReconcileFailed(false)
    } catch {
      if (alive.current) setReconcileFailed(true)
    } finally {
      submitting.current = false
      if (alive.current) setBusy(false)
    }
  }
  const transition = async (action: RoundTransition) => {
    if (!canAct || submitting.current || roundAction(round.status) !== action) return
    submitting.current = true
    setBusy(true)
    setMessage(null)
    let receipt: typeof message
    try {
      await mutation.mutateAsync(action)
      receipt = { kind: 'success', text: action === 'open' ? 'Runden er åpnet.' : action === 'complete' ? 'Runden er fullført.' : 'Runden er låst.' }
    } catch (failure) {
      receipt = { kind: 'error', text: lifecycleFailure(failure) }
    }
    try {
      await reconcile()
      if (alive.current) setReconcileFailed(false)
    } catch {
      if (alive.current) setReconcileFailed(true)
    } finally {
      submitting.current = false
      if (alive.current) { setMessage(receipt); setBusy(false) }
    }
  }
  return {
    round, opening: opening.data, completion: completion.data,
    version: `${round.updated_at}:${readiness.dataUpdatedAt}`,
    loading, refreshing, error, inconsistent, canAct, busy, message, reconcileFailed,
    refresh, transition,
  }
}
