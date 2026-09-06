import { useEffect, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tournamentApi } from '../../../api/tournaments'
import type { Tournament } from '../../../api/types'
import { useAuth } from '../../auth/authContext'
import { archiveFailure } from './archiveState'
import { reconcileTournamentClosure } from '../reconcileTournamentClosure'

export interface ArchiveProps {
  tournament: Tournament
  authorityRefreshing: boolean
  snapshotVersion: string
}

export function useTournamentArchive({ tournament, authorityRefreshing }: ArchiveProps) {
  const auth = useAuth()
  const client = useQueryClient()
  const alive = useRef(false)
  const submitting = useRef(false)
  const [busy, setBusy] = useState(false)
  const [refreshFailed, setRefreshFailed] = useState(false)
  const [message, setMessage] = useState<{ error: boolean; text: string } | null>(null)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const canAct = !!auth.session && !auth.loading && !auth.error && !authorityRefreshing
    && !busy && !refreshFailed && tournament.status === 'completed'
  const mutation = useMutation({
    mutationFn: () => {
      if (!auth.session) throw new Error('Økten mangler.')
      return tournamentApi.archive(tournament.id, tournament.updated_at, auth.session.csrf_token)
    }, retry: false,
  })
  const run = async (archive: boolean) => {
    if (submitting.current || (archive && !canAct)) return
    submitting.current = true
    setBusy(true)
    let receipt: typeof message = null
    if (archive) {
      setMessage(null)
      try { await mutation.mutateAsync(); receipt = { error: false, text: 'Turneringen er arkivert.' } }
      catch (error) { receipt = { error: true, text: archiveFailure(error) } }
    }
    try {
      await reconcileTournamentClosure(client, auth.session?.user_id ?? '', tournament.id)
      if (alive.current) setRefreshFailed(false)
    } catch { if (alive.current) setRefreshFailed(true) }
    finally {
      submitting.current = false
      if (alive.current) { setBusy(false); if (receipt) setMessage(receipt) }
    }
  }
  return { canAct, busy, refreshFailed, message, archive: () => run(true), refresh: () => run(false) }
}
