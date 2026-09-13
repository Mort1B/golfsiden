import type { QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { api } from '../../api/client'
import { publishSessionTransition } from '../auth/sessionTransition'

export function sameRecoverySession(client: QueryClient, initial: AuthSession | null | undefined): boolean {
  const current = client.getQueryData<AuthSession | null>(authKeys.session)
  return current?.user_id === initial?.user_id && current?.csrf_token === initial?.csrf_token
}

export async function refreshRecoverySession(client: QueryClient, initial: AuthSession | null | undefined): Promise<void> {
  if (!sameRecoverySession(client, initial)) return
  await client.cancelQueries({ queryKey: authKeys.session })
  if (!sameRecoverySession(client, initial)) return
  const refreshed = await api.session()
  if (!sameRecoverySession(client, initial)) return
  publishSessionTransition(client, refreshed)
}
