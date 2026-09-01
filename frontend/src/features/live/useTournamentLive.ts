import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { invalidateLiveQueries } from '../../api/liveInvalidation'
import { subscribeTournamentLive } from '../../api/tournamentLive'
import { useAuth } from '../auth/authContext'

export function useTournamentLive(tournamentId: string): void {
  const queryClient = useQueryClient()
  const userId = useAuth().session?.user_id ?? ''

  useEffect(() => subscribeTournamentLive(userId, tournamentId, () => {
    void invalidateLiveQueries(queryClient, userId)
  }), [queryClient, tournamentId, userId])
}
