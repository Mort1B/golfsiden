import { useEffect, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { authKeys, type AuthSession } from '../../api/auth'
import { api } from '../../api/client'
import { ApiHttpError } from '../../api/http'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import { publishSessionTransition } from '../auth/sessionTransition'

function message(error: unknown): string {
  if (!(error instanceof ApiHttpError)) return 'Vi fikk ikke bekreftet endringen. Oppdater profilen før du prøver igjen. Ved passordbytte kan du måtte logge inn med det nye passordet.'
  if (error.code === 'current_password_incorrect') return 'Nåværende passord er ikke riktig.'
  if (error.code === 'username_unavailable') return 'Brukernavnet er opptatt. Velg et annet.'
  if (error.code === 'profile_stale') return 'Profilen er endret siden du åpnet den. Oppdater profilen før du prøver igjen.'
  if (error.code === 'profile_inactive') return 'Spillerprofilen er deaktivert. Kontakt en administrator for å endre handicap.'
  if (error.status === 401) return 'Økten er utløpt. Logg inn igjen.'
  if (error.status === 429) return 'For mange forsøk. Vent litt før du prøver igjen.'
  return 'Endringen ble avvist. Kontroller feltene og prøv igjen.'
}

export function useProfileActions(session: AuthSession) {
  const client = useQueryClient()
  const navigate = useNavigate()
  const alive = useRef(false)
  const busy = useRef(false)
  const [pending, setPending] = useState(false)
  const [feedback, setFeedback] = useState<{ error: boolean; text: string } | null>(null)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const mutation = useMutation({ mutationFn: (operation: () => Promise<unknown>) => operation(), gcTime: 0 })
  const sameSession = () => {
    const current = client.getQueryData<AuthSession | null>(authKeys.session)
    return current?.user_id === session.user_id && current.csrf_token === session.csrf_token
  }
  const current = () => alive.current && sameSession()
  const run = async (operation: () => Promise<unknown>, password = false): Promise<boolean> => {
    if (busy.current) return false
    busy.current = true
    setPending(true)
    setFeedback(null)
    try {
      await mutation.mutateAsync(operation)
      if (!sameSession()) return false
      if (password) {
        await client.cancelQueries({ queryKey: authKeys.session })
        if (!sameSession()) return false
        // Leave the protected route before clearing identity, so its ordinary
        // sign-in redirect cannot replace the password-change confirmation.
        if (alive.current) navigate('/login', { replace: true, flushSync: true, state: { profilePasswordChanged: true } })
        publishSessionTransition(client, null)
        return true
      }
      // Refresh session names/linkage without treating a same-user edit as logout.
      const refreshed = await api.session()
      if (!sameSession()) return false
      publishSessionTransition(client, refreshed)
      if (refreshed?.user_id !== session.user_id) return false
      await client.invalidateQueries({ queryKey: privateWorkspaceKeys.user(session.user_id) }, { throwOnError: true })
      if (!current()) return false
      setFeedback({ error: false, text: 'Endringen er lagret og profilen er oppdatert.' })
      return true
    } catch (error) {
      if (current()) setFeedback({ error: true, text: message(error) })
      return false
    } finally {
      // Remove credential-bearing mutation variables from the inactive cache.
      mutation.reset()
      busy.current = false
      if (current()) setPending(false)
    }
  }
  return { run, pending, feedback }
}
