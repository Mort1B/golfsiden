import { useEffect, useRef, useState, type ReactNode } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAuth } from '../../auth/authContext'
import { MatchQueueContext } from './context'
import { MatchRuntime } from './runtime'
export function MatchQueueProvider({ children }: { children: ReactNode }) {
  const { session } = useAuth(), identity = session ? `${session.user_id}:${session.csrf_token}` : ''
  const current = useRef(identity); current.current = identity
  return <SessionQueue key={identity} account={session?.user_id ?? ''} csrf={session?.csrf_token ?? ''} owns={() => current.current === identity}>{children}</SessionQueue>
}
function SessionQueue({ children, account, csrf, owns }: { children: ReactNode; account: string; csrf: string; owns: () => boolean }) {
  const client = useQueryClient(), [runtime] = useState(() => new MatchRuntime(account, csrf, client, owns))
  useEffect(() => account ? runtime.start() : undefined, [account, runtime])
  return <MatchQueueContext value={runtime}>{children}</MatchQueueContext>
}
