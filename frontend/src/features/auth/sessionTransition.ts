import type { QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { clearPrivateWorkspace } from '../../api/privateWorkspace'
import { acceptIdentityPublication, identityVersion, settledIdentityVersion, waitForIdentity } from '../../api/identityQueue'

const readVersions = new WeakMap<QueryClient, number>()
function nextRead(client: QueryClient): number {
  const next = (readVersions.get(client) ?? 0) + 1
  readVersions.set(client, next)
  return next
}

export async function resolveSessionTransition(
  queryClient: QueryClient,
  requestSession: () => Promise<AuthSession | null>,
  signal?: AbortSignal,
): Promise<AuthSession | null> {
  const initial = queryClient.getQueryData<AuthSession | null>(authKeys.session)
  const intent = identityVersion()
  const read = nextRead(queryClient)
  if (intent !== settledIdentityVersion()) await waitForIdentity()
  signal?.throwIfAborted()
  if (intent !== identityVersion() || read !== readVersions.get(queryClient)) {
    return queryClient.getQueryData<AuthSession | null>(authKeys.session) ?? null
  }
  let session: AuthSession | null
  try { session = await requestSession() }
  catch (error) {
    if (intent === identityVersion()) throw error
    await waitForIdentity()
    signal?.throwIfAborted()
    return queryClient.getQueryData<AuthSession | null>(authKeys.session) ?? null
  }
  signal?.throwIfAborted()
  if (intent !== identityVersion()) await waitForIdentity()
  const current = queryClient.getQueryData<AuthSession | null>(authKeys.session)
  if (intent !== identityVersion() || read !== readVersions.get(queryClient)
    || current?.user_id !== initial?.user_id || current?.csrf_token !== initial?.csrf_token) {
    return current ?? null
  }
  clearForIdentityTransition(queryClient, session)
  return session
}

export function publishSessionTransition(
  queryClient: QueryClient,
  session: AuthSession | null,
): void {
  nextRead(queryClient)
  acceptIdentityPublication()
  clearForIdentityTransition(queryClient, session)
  queryClient.setQueryData(authKeys.session, session)
}

function clearForIdentityTransition(queryClient: QueryClient, next: AuthSession | null): void {
  const current = queryClient.getQueryData<AuthSession | null>(authKeys.session)
  if (current?.user_id === next?.user_id && current?.csrf_token !== next?.csrf_token) {
    queryClient.removeQueries({ predicate: query => query.queryKey[0] === 'private-workspace' && (query.queryKey[2] === 'tournaments' && query.queryKey[4] === 'fantasy' || query.queryKey[2] === 'rounds' && query.queryKey[4] === 'awards') })
  }
  if (next === null || current === undefined || current?.user_id !== next?.user_id) {
    clearPrivateWorkspace(queryClient)
  }
}
