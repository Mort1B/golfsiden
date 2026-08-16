import type { QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { clearPrivateWorkspace } from '../../api/privateWorkspace'

export async function resolveSessionTransition(
  queryClient: QueryClient,
  requestSession: () => Promise<AuthSession | null>,
): Promise<AuthSession | null> {
  const session = await requestSession()
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
