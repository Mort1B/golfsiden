import type { QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { api } from '../../api/client'
import { publishSessionTransition, resolveSessionTransition } from '../auth/sessionTransition'
import { identityVersion } from '../../api/identityQueue'

export function sameRecoverySession(client: QueryClient, initial: AuthSession | null | undefined): boolean {
  const current = client.getQueryData<AuthSession | null>(authKeys.session)
  return current?.user_id === initial?.user_id && current?.csrf_token === initial?.csrf_token
}

export async function refreshRecoverySession(client: QueryClient, initial: AuthSession | null | undefined): Promise<void> {
  const intent = identityVersion()
  if (!sameRecoverySession(client, initial)) return
  await client.cancelQueries({ queryKey: authKeys.session })
  if (intent !== identityVersion() || !sameRecoverySession(client, initial)) return
  const refreshed = await resolveSessionTransition(client, api.session)
  if (intent !== identityVersion() || !sameRecoverySession(client, initial)) return
  publishSessionTransition(client, refreshed)
}
