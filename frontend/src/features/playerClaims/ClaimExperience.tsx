import { useEffect, useId, useRef, useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { authKeys, type AuthSession } from '../../api/auth'
import { api } from '../../api/client'
import { isCanonicalUuid } from '../../api/decoder'
import { ApiHttpError } from '../../api/http'
import { playerClaimsApi } from '../../api/playerClaims'
import { useAuth } from '../auth/authContext'
import { PASSWORD_GUIDANCE, passwordValidationMessage } from '../auth/password'
import { publishSessionTransition } from '../auth/sessionTransition'
import { USERNAME_HTML_PATTERN } from '../auth/username'
import { sameRecoverySession } from '../recovery/refreshRecoverySession'
import { recoveryExpiry } from '../recovery/messages'
import { claimMessage, invalidClaimMessage } from './messages'
import './playerClaims.css'

export function ClaimExperience({ claimId, fragment }: { claimId: string; fragment: string }) {
  const [captured, setCaptured] = useState(() => /^#token=([A-Za-z0-9_-]{43})$/.exec(fragment)?.[1] ?? null)
  const secret = useRef(captured)
  const instance = useId()
  const client = useQueryClient()
  const auth = useAuth()
  const alive = useRef(false)
  const busy = useRef(false)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [pending, setPending] = useState(false)
  const [feedback, setFeedback] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)
  const [invalid, setInvalid] = useState(false)
  const previewKey = ['player-claim-preview', claimId, instance]
  const valid = isCanonicalUuid(claimId) && captured !== null
  useEffect(() => {
    alive.current = true
    window.history.replaceState(window.history.state, '', `${window.location.pathname}${window.location.search}`)
    return () => { alive.current = false }
  }, [])
  const preview = useQuery({ queryKey: previewKey, enabled: valid && !success && !invalid,
    retry: false, gcTime: 0, staleTime: Infinity, refetchOnWindowFocus: false, refetchOnReconnect: false,
    queryFn: async ({ signal }) => {
      if (!secret.current) throw new Error(invalidClaimMessage)
      const result = await playerClaimsApi.preview(claimId, secret.current, signal)
      signal.throwIfAborted()
      return result
    },
  })
  const mutation = useMutation({ mutationFn: (operation: () => Promise<void>) => operation(), retry: false, gcTime: 0 })
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (busy.current || !preview.data || !secret.current || auth.loading || auth.session) return
    const validation = !/^[A-Za-z0-9_-]{3,32}$/.test(username.trim()) ? 'Bruk 3–32 bokstaver, tall, bindestrek eller understrek i brukernavnet.' : passwordValidationMessage(password)
    if (validation) { setFeedback(validation); return }
    const initial = client.getQueryData<AuthSession | null>(authKeys.session)
    if (initial) { setFeedback('Du er allerede innlogget. Logg ut før du tar i bruk denne kontoen.'); return }
    const token = secret.current
    const supplied = password
    const accountName = username.trim()
    const expected = preview.data
    busy.current = true; setPending(true); setFeedback(null); setPassword('')
    try {
      await client.cancelQueries({ queryKey: authKeys.session })
      if (!alive.current || !sameRecoverySession(client, initial)) return
      await mutation.mutateAsync(async () => {
        const receipt = await playerClaimsApi.register(claimId, token, accountName, supplied, expected)
        if (!alive.current || !sameRecoverySession(client, initial)) return
        await client.cancelQueries({ queryKey: authKeys.session })
        if (!alive.current || !sameRecoverySession(client, initial)) return
        secret.current = null; setCaptured(null)
        client.removeQueries({ queryKey: previewKey, exact: true })
        publishSessionTransition(client, receipt.session)
        setSuccess(receipt.tournament_id)
      })
    } catch (error) {
      if (!alive.current || !sameRecoverySession(client, initial)) return
      setFeedback(claimMessage(error))
      if (error instanceof ApiHttpError && ['claim_invalid', 'claim_unavailable'].includes(error.code)) {
        setInvalid(true); secret.current = null; setCaptured(null)
        client.removeQueries({ queryKey: previewKey, exact: true })
      }
    } finally { mutation.reset(); busy.current = false; if (alive.current) setPending(false) }
  }
  const signOut = async () => {
    if (busy.current) return
    busy.current = true; setPending(true); setFeedback(null)
    const initial = client.getQueryData<AuthSession | null>(authKeys.session)
    try {
      if (!initial) return
      await api.logout(initial.csrf_token)
      if (alive.current && sameRecoverySession(client, initial)) publishSessionTransition(client, null)
    } catch (error) {
      if (alive.current && sameRecoverySession(client, initial)) {
        if (error instanceof ApiHttpError && error.status === 401) publishSessionTransition(client, null)
        else setFeedback(claimMessage(error))
      }
    }
    finally { busy.current = false; if (alive.current) setPending(false) }
  }
  const unavailable = !valid || invalid || (preview.error instanceof ApiHttpError && ['claim_invalid', 'claim_unavailable'].includes(preview.error.code))
  return <section className="sign-in-panel" aria-labelledby="claim-heading">
    <p className="brand">Guttas Golf</p><h1 id="claim-heading">Ta i bruk spillerkontoen</h1>
    {success ? <><p role="status">Kontoen er klar. Du er logget inn som spilleren arrangøren opprettet.</p><Link to={`/tournaments/${success}`}>Åpne turneringen</Link></> : unavailable ? <><p role="alert">{invalidClaimMessage}</p><Link to="/login">Til innlogging</Link></> : <>
      {preview.isPending && <p role="status">Kontrollerer lenken …</p>}
      {preview.error && <><p role="alert">{claimMessage(preview.error)}</p><button disabled={preview.isFetching} onClick={() => void preview.refetch()}>Prøv igjen</button></>}
      {preview.data && !preview.error && <>
        <p>Konto for <strong>{preview.data.player.display_name}</strong> i <strong>{preview.data.tournament.name}</strong>.</p>
        <p>Lenken kan brukes én gang og utløper {recoveryExpiry(preview.data.expires_at)}.</p>
        {auth.loading ? <p role="status">Kontrollerer innlogging …</p> : auth.session ? <><p>Du er innlogget som {auth.session.display_name}. Logg ut for å ta i bruk den nye spillerkontoen. Kontoer slås ikke sammen.</p><button disabled={pending} onClick={() => void signOut()}>Logg ut og fortsett</button></> : <form onSubmit={event => void submit(event)} aria-busy={pending}>
          <label><span>Brukernavn</span><input autoComplete="username" required pattern={USERNAME_HTML_PATTERN} minLength={3} maxLength={32} value={username} disabled={pending} onChange={event => setUsername(event.target.value)} /></label>
          <p id="claim-password-help">{PASSWORD_GUIDANCE}</p>
          <label><span>Passord</span><input type="password" autoComplete="new-password" aria-describedby="claim-password-help" required value={password} disabled={pending} onChange={event => setPassword(event.target.value)} /></label>
          <button type="submit" disabled={pending}>{pending ? 'Oppretter konto …' : 'Ta i bruk kontoen'}</button>
        </form>}
        {feedback && <p role="alert">{feedback}</p>}
      </>}
    </>}
  </section>
}
