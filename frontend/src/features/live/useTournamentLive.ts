import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { handleTournamentLiveSignal } from '../../api/liveInvalidation'
import { subscribeTournamentLive } from '../../api/tournamentLive'
import { useAuth } from '../auth/authContext'

export function useTournamentLive(tournamentId: string): void {
  const queryClient = useQueryClient()
  const userId = useAuth().session?.user_id ?? ''

  useEffect(() => subscribeTournamentLive(userId, tournamentId, (signal) => {
    void handleTournamentLiveSignal(queryClient, userId, signal)
  }), [queryClient, tournamentId, userId])
}
