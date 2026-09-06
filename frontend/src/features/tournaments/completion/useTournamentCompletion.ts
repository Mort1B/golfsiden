import { useEffect, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tournamentApi } from '../../../api/tournaments'
import type { Round, Tournament } from '../../../api/types'
import { useAuth } from '../../auth/authContext'
import { completionFailure, completionReadiness, reconcileCompletion } from './completionState'

export interface CompletionProps {
  tournament: Tournament
  rounds: { data: Round[] | undefined; pending: boolean; error: Error | null }
  authorityRefreshing: boolean
  snapshotVersion: string
}

export function useTournamentCompletion({ tournament, rounds, authorityRefreshing }: CompletionProps) {
  const auth = useAuth()
  const client = useQueryClient()
  const alive = useRef(false)
  const submitting = useRef(false)
  const [busy, setBusy] = useState(false)
  const [refreshFailed, setRefreshFailed] = useState(false)
  const [message, setMessage] = useState<{ error: boolean; text: string } | null>(null)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const readiness = completionReadiness(tournament, rounds.data)
  const canAct = !!auth.session && !auth.loading && !auth.error && !authorityRefreshing
    && !rounds.pending && !rounds.error && !busy && !refreshFailed && readiness.ready
  const mutation = useMutation({
    mutationFn: () => {
      if (!auth.session) throw new Error('Økten mangler.')
      return tournamentApi.complete(tournament.id, tournament.updated_at, auth.session.csrf_token)
    }, retry: false,
  })
  const run = async (complete: boolean) => {
    if (submitting.current || (complete && !canAct)) return
    submitting.current = true
    setBusy(true)
    let receipt: typeof message = null
    if (complete) {
      setMessage(null)
      try { await mutation.mutateAsync(); receipt = { error: false, text: 'Turneringen er fullført.' } }
      catch (error) { receipt = { error: true, text: completionFailure(error) } }
    }
    try {
      await reconcileCompletion(client, auth.session?.user_id ?? '', tournament.id)
      if (alive.current) setRefreshFailed(false)
    } catch { if (alive.current) setRefreshFailed(true) }
    finally {
      submitting.current = false
      if (alive.current) { setBusy(false); if (receipt) setMessage(receipt) }
    }
  }
  return { readiness, canAct, busy, refreshFailed, message, complete: () => run(true), refresh: () => run(false) }
}
