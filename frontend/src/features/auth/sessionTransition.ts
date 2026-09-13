import type { QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { clearPrivateWorkspace } from '../../api/privateWorkspace'

export async function resolveSessionTransition(
  queryClient: QueryClient,
  requestSession: () => Promise<AuthSession | null>,
  signal?: AbortSignal,
): Promise<AuthSession | null> {
  const initial = queryClient.getQueryData<AuthSession | null>(authKeys.session)
  const session = await requestSession()
  signal?.throwIfAborted()
  const current = queryClient.getQueryData<AuthSession | null>(authKeys.session)
  if (current?.user_id !== initial?.user_id || current?.csrf_token !== initial?.csrf_token) {
    return current ?? null
  }
  clearForIdentityTransition(queryClient, session)
  return session
}

export function publishSessionTransition(
  queryClient: QueryClient,
  session: AuthSession | null,
): void {
  clearForIdentityTransition(queryClient, session)
  queryClient.setQueryData(authKeys.session, session)
}

function clearForIdentityTransition(queryClient: QueryClient, next: AuthSession | null): void {
  const current = queryClient.getQueryData<AuthSession | null>(authKeys.session)
  if (current === undefined || current?.user_id !== next?.user_id) {
    clearPrivateWorkspace(queryClient)
  }
}
