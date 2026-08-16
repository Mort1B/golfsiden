import { useEffect, type ReactNode } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../../api/client'
import { authKeys } from '../../api/auth'
import { AuthContext } from './authContext'
import { ApiHttpError } from '../../api/http'
import { tournamentKeys } from '../../api/tournaments'

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient()
  const sessionQuery = useQuery({
    queryKey: authKeys.session,
    queryFn: api.session,
    retry: false,
    staleTime: 30_000,
  })

  useEffect(() => {
    if (sessionQuery.isSuccess && sessionQuery.data === null) {
      queryClient.removeQueries({ queryKey: tournamentKeys.mineRoot })
    }
  }, [queryClient, sessionQuery.data, sessionQuery.isSuccess])

  const clearProtectedState = () => {
    queryClient.setQueryData(authKeys.session, null)
    queryClient.removeQueries({ queryKey: tournamentKeys.mineRoot })
    queryClient.removeQueries({
      predicate: (query) => query.queryKey[0] === 'rounds' && query.queryKey[2] === 'score-access',
    })
  }

  return (
    <AuthContext.Provider value={{
      session: sessionQuery.data ?? null,
      loading: sessionQuery.isPending,
      error: sessionQuery.error,
      signIn: async (username, password) => {
        const session = await api.login(username, password)
        queryClient.removeQueries({ queryKey: tournamentKeys.mineRoot })
        queryClient.setQueryData(authKeys.session, session)
        return session
      },
      establishSession: (session) => {
        queryClient.removeQueries({ queryKey: tournamentKeys.mineRoot })
        queryClient.setQueryData(authKeys.session, session)
      },
      signOut: async () => {
        const session = sessionQuery.data
        try {
          if (session) await api.logout(session.csrf_token)
          clearProtectedState()
        } catch (error) {
          if (error instanceof ApiHttpError && error.status === 401) {
            clearProtectedState()
            return
          }
          throw error
        }
      },
      retry: async () => { await sessionQuery.refetch() },
    }}>
      {children}
    </AuthContext.Provider>
  )
}
