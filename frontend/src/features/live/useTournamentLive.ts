import { useEffect, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { handleTournamentLiveSignal } from '../../api/liveInvalidation'
import { subscribeTournamentLive } from '../../api/tournamentLive'
import { useAuth } from '../auth/authContext'

export function useTournamentLive(tournamentId: string): boolean {
  const queryClient = useQueryClient()
  const userId = useAuth().session?.user_id ?? ''
  const scope = `${userId}:${tournamentId}`
  const [disconnectedScope, setDisconnectedScope] = useState<string | null>(null)

  useEffect(() => subscribeTournamentLive(userId, tournamentId, (signal) => {
    if (signal === 'error') setDisconnectedScope(scope)
    if (signal === 'open') setDisconnectedScope(null)
    void handleTournamentLiveSignal(queryClient, userId, signal)
  }), [queryClient, tournamentId, userId, scope])
  return disconnectedScope === scope
}
