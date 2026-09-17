import { ScoreDraftContext } from '../recovery/context'
import { ScoreDraftStore } from '../recovery/store'
import { useEffect, useRef, useState, type ReactNode } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAuth } from '../../auth/authContext'
import { ScoreQueueContext } from './context'
import { QueueRuntime } from './runtime'

export function ScoreQueueProvider({ children }: { children: ReactNode }) {
  const { session } = useAuth()
  const accountId = session?.user_id ?? ''
  const [drafts, setDrafts] = useState(() => new ScoreDraftStore(accountId))
  if (drafts.accountId !== accountId) setDrafts(new ScoreDraftStore(accountId))
  const identity = session ? `${session.user_id}:${session.csrf_token}` : ''
  const current = useRef(identity)
  current.current = identity
  return <ScoreDraftContext value={drafts}><SessionQueue key={identity} accountId={session?.user_id ?? ''} csrfToken={session?.csrf_token ?? ''}
    ownsSession={() => current.current === identity}>{children}</SessionQueue></ScoreDraftContext>
}
function SessionQueue({ accountId, csrfToken, ownsSession, children }: {
  accountId: string; csrfToken: string; ownsSession: () => boolean; children: ReactNode
}) {
  const client = useQueryClient()
  const [runtime] = useState(() => new QueueRuntime(accountId, csrfToken, client, ownsSession))
  useEffect(() => accountId ? runtime.start() : undefined, [accountId, runtime])
  return <ScoreQueueContext value={runtime}>{children}</ScoreQueueContext>
}
