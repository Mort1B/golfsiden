import { useEffect, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { ApiHttpError } from '../../api/http'
import { playerClaimsApi, type ClaimReceipt } from '../../api/playerClaims'
import { invalidateLiveQueries } from '../../api/liveInvalidation'
import { claimMessage } from './messages'
export type PlayerAction = 'create' | 'reissue' | 'revoke' | 'withdraw'
export function usePlayerManagement(tournament: string, session: AuthSession) {
  const client = useQueryClient()
  const alive = useRef(false)
  const busy = useRef(false)
  const [pending, setPending] = useState(false)
  const [receipt, setReceipt] = useState<ClaimReceipt | null>(null)
  const [feedback, setFeedback] = useState<{ error: boolean; text: string; code?: string } | null>(null)
  const [uncertain, setUncertain] = useState(false)
  const mutation = useMutation({ mutationFn: (operation: () => Promise<ClaimReceipt | void>) => operation(), gcTime: 0, retry: false })
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const current = () => {
    const cached = client.getQueryData<AuthSession | null>(authKeys.session)
    return alive.current && cached?.user_id === session.user_id && cached.csrf_token === session.csrf_token
  }
  const refresh = () => invalidateLiveQueries(client, session.user_id)
  const run = async (action: PlayerAction, playerId = '', name = '', handicap = 0): Promise<boolean> => {
    if (busy.current || !current()) return false
    busy.current = true; setPending(true); setFeedback(null); setReceipt(null)
    try {
      const result = await mutation.mutateAsync(() => {
        if (action === 'create') return playerClaimsApi.create(tournament, name, handicap, session.csrf_token)
        if (action === 'reissue') return playerClaimsApi.reissue(tournament, playerId, session.csrf_token)
        return playerClaimsApi[action](tournament, playerId, session.csrf_token)
      })
      if (!current()) return false
      if (result) setReceipt(result)
      else setFeedback({ error: false, text: action === 'withdraw' ? 'Spilleren er fjernet fra aktiv deltakelse. Historikk og eksisterende tilgang er bevart.' : 'Kontolenken er tilbakekalt.' })
      setUncertain(false)
      await refresh()
      return current()
    } catch (error) {
      if (!current()) return false
      const unknownCreate = action === 'create' && (!(error instanceof ApiHttpError) || error.status >= 500)
      setUncertain(unknownCreate)
      setFeedback({ error: true, text: unknownCreate ? 'Vi fikk ikke bekreftet opprettelsen. Spilleren kan være opprettet. Kontroller den oppdaterte listen og lag en ny lenke for spilleren før du eventuelt oppretter på nytt.' : claimMessage(error), code: error instanceof ApiHttpError ? error.code : undefined })
      await refresh()
      return false
    } finally { mutation.reset(); busy.current = false; if (current()) setPending(false) }
  }
  return { pending, receipt, feedback, uncertain, current, run, hide: () => setReceipt(null), acknowledge: () => setUncertain(false) }
}
