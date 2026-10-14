import { useEffect, useState, type ReactNode } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../../api/client'
import { authKeys, type AuthSession } from '../../api/auth'
import { AuthContext } from './authContext'
import { ApiHttpError } from '../../api/http'
import { publishSessionTransition, resolveSessionTransition } from './sessionTransition'
import { acceptIdentityPublication, identityVersion, observeIdentity } from '../../api/identityQueue'
import { readCookieSession } from '../../api/authRequests'

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient()
  const [reconciliationError, setReconciliationError] = useState<Error | null>(null)
  useEffect(() => {
    let origin: AuthSession | null | undefined
    const same = (left: AuthSession | null | undefined, right: AuthSession | null | undefined) =>
      left?.user_id === right?.user_id && left?.csrf_token === right?.csrf_token
    return observeIdentity({
      started: () => {
        origin = queryClient.getQueryData<AuthSession | null>(authKeys.session)
        setReconciliationError(null)
      },
      settled: async (known, version) => {
        const current = () => queryClient.getQueryData<AuthSession | null>(authKeys.session)
        if (!same(current(), origin) && (known === undefined || !same(current(), known))) return
        try {
          const next = known === undefined ? await readCookieSession() : known
          if (version !== identityVersion()) return
          if (same(current(), origin) || same(current(), next)) {
            acceptIdentityPublication(version)
            publishSessionTransition(queryClient, next)
          }
        } catch (error) {
          if (version !== identityVersion()) return
          acceptIdentityPublication(version)
          publishSessionTransition(queryClient, null)
          setReconciliationError(error instanceof Error ? error : new Error('Innloggingen kunne ikke kontrolleres.'))
        }
      },
    })
  }, [queryClient])
  const sessionQuery = useQuery({
    queryKey: authKeys.session,
    queryFn: ({ signal }) => resolveSessionTransition(queryClient, api.session, signal),
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
      error: reconciliationError ?? (sessionQuery.data === undefined ? sessionQuery.error : null),
      signIn: async (username, password) => {
        const pending = api.login(username, password)
        const version = identityVersion()
        const session = await pending
        if (version === identityVersion()) establishProtectedState(session)
        return session
      },
      establishSession: (session) => {
        establishProtectedState(session)
      },
      signOut: async () => {
        const pending = api.logout(sessionQuery.data?.csrf_token)
        const version = identityVersion()
        try {
          await pending
          if (version === identityVersion()) clearProtectedState()
        } catch (error) {
          if (error instanceof ApiHttpError && error.status === 401) {
            if (version === identityVersion()) clearProtectedState()
            return
          }
          throw error
        }
      },
      retry: async () => {
        const result = await sessionQuery.refetch()
        if (!result.error) setReconciliationError(null)
      },
    }}>
      {children}
    </AuthContext.Provider>
  )
}
