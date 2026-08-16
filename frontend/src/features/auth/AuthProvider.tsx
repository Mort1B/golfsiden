import type { ReactNode } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../../api/client'
import { authKeys } from '../../api/auth'
import { AuthContext } from './authContext'
import { ApiHttpError } from '../../api/http'
import { publishSessionTransition, resolveSessionTransition } from './sessionTransition'

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient()
  const sessionQuery = useQuery({
    queryKey: authKeys.session,
    queryFn: () => resolveSessionTransition(queryClient, api.session),
    retry: false,
    staleTime: 30_000,
  })

  const clearProtectedState = () => {
    publishSessionTransition(queryClient, null)
  }

  const establishProtectedState = (session: NonNullable<typeof sessionQuery.data>) => {
    publishSessionTransition(queryClient, session)
  }

  return (
    <AuthContext.Provider value={{
      session: sessionQuery.data ?? null,
      loading: sessionQuery.isPending,
      error: sessionQuery.data === undefined ? sessionQuery.error : null,
      signIn: async (username, password) => {
        const session = await api.login(username, password)
        establishProtectedState(session)
        return session
      },
      establishSession: (session) => {
        establishProtectedState(session)
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
