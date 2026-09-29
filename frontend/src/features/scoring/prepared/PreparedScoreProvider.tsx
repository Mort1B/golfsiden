import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAuth } from '../../auth/authContext'
import { eligiblePrepared, freshPreparation, type PreparedSelection } from './eligibility'
import { PreparedContext } from './context'
export function PreparedScoreProvider({ children }: { children: ReactNode }) {
  const { session } = useAuth(), client = useQueryClient()
  const identity = session ? `${session.user_id}:${session.csrf_token}` : ''
  const [state, setState] = useState<{ identity: string; target: PreparedSelection | null }>({ identity, target: null })
  const owned = useRef({ identity, target: null as PreparedSelection | null, barrier: 0 })
  if (state.identity !== identity) {
    owned.current = { identity, target: null, barrier: state.identity ? Date.now() : 0 }
    setState({ identity, target: null })
  }
  const prepared = state.identity === identity && state.target && eligiblePrepared(client, session, state.target) ? state.target : null
  useEffect(() => {
    const inspect = () => {
      const prior = owned.current
      if (prior.identity !== identity || !prior.target || eligiblePrepared(client, session, prior.target)) return
      // Inspect synchronously: a later SSE event may erase a terminal error before
      // React processes its state updates. The marker must stay revoked.
      owned.current = { identity, target: null, barrier: Date.now() }
      setState({ identity, target: null })
    }
    const stop = client.getQueryCache().subscribe(inspect)
    let timer: ReturnType<typeof setTimeout>
    const expiry = () => { inspect(); const left = session ? Date.parse(session.expires_at) - Date.now() : 0
      if (left > 0) timer = setTimeout(expiry, Math.min(left + 1, 2_000_000_000)) }
    expiry()
    return () => { stop(); clearTimeout(timer) }
  }, [client, identity, session])
  const prepare = useCallback((target: PreparedSelection) => {
    const prior = owned.current
    if (prior.identity !== identity) return
    const sameCard = prior.target?.tournamentId === target.tournamentId && prior.target.roundId === target.roundId
      && prior.target.owner.type === target.owner.type && prior.target.owner.id === target.owner.id
    // Moving holes on the same retained card does not invent new authority.
    if (!(sameCard && eligiblePrepared(client, session, target)) && !freshPreparation(client, session, target, prior.barrier)) return
    if (sameCard && prior.target?.holeNumber === target.holeNumber) return
    owned.current = { ...prior, target }; setState({ identity, target })
  }, [client, identity, session])
  const available = () => owned.current.identity === identity && owned.current.target && eligiblePrepared(client, session, owned.current.target) ? owned.current.target : null
  return <PreparedContext value={{ prepared, prepare, available }}>{children}</PreparedContext>
}
