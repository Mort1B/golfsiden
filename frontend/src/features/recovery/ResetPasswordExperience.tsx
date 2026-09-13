import { useEffect, useId, useRef, useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { authKeys, type AuthSession } from '../../api/auth'
import { isCanonicalUuid } from '../../api/decoder'
import { ApiHttpError } from '../../api/http'
import { passwordRecoveryApi, recoveryTokenPattern } from '../../api/passwordRecovery'
import { PASSWORD_GUIDANCE, passwordValidationMessage } from '../auth/password'
import { invalidRecoveryMessage, recoveryExpiry, recoveryMessage } from './messages'
import { refreshRecoverySession, sameRecoverySession } from './refreshRecoverySession'
import './recovery.css'

export function ResetPasswordExperience({ grantId, fragment }: { grantId: string; fragment: string }) {
  // Capture before clearing history, including both StrictMode initial renders.
  const [captured, setCaptured] = useState(() => /^#token=([A-Za-z0-9_-]{43})$/.exec(fragment)?.[1] ?? null)
  const token = useRef(captured)
  const instance = useId()
  const client = useQueryClient()
  const alive = useRef(false)
  const busy = useRef(false)
  const [password, setPassword] = useState('')
  const [repeat, setRepeat] = useState('')
  const [pending, setPending] = useState(false)
  const [success, setSuccess] = useState(false)
  const [feedback, setFeedback] = useState<string | null>(null)
  const [invalid, setInvalid] = useState(false)
  const previewKey = ['password-recovery-preview', grantId, instance]
  const valid = isCanonicalUuid(grantId) && captured !== null && recoveryTokenPattern.test(captured)
  useEffect(() => {
    alive.current = true
    window.history.replaceState(window.history.state, '', `${window.location.pathname}${window.location.search}`)
    return () => { alive.current = false }
  }, [])
  const preview = useQuery({
    queryKey: previewKey, enabled: valid && !success && !invalid, retry: false, gcTime: 0,
    staleTime: Infinity, refetchOnWindowFocus: false, refetchOnReconnect: false,
    queryFn: async ({ signal }) => {
      const secret = token.current
      if (!secret) throw new Error(invalidRecoveryMessage)
      const result = await passwordRecoveryApi.preview(grantId, secret)
      signal.throwIfAborted()
      return result
    },
  })
  const mutation = useMutation({ mutationFn: (operation: () => Promise<void>) => operation(), retry: false, gcTime: 0 })
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (busy.current || !preview.data || !token.current) return
    const validation = passwordValidationMessage(password) ?? (password !== repeat ? 'Passordene må være like.' : null)
    if (validation) { setFeedback(validation); return }
    busy.current = true; setPending(true); setFeedback(null)
    const secret = token.current
    const supplied = password
    const confirmation = repeat
    const initial = client.getQueryData<AuthSession | null>(authKeys.session)
    try {
      // Cancel pre-reset reads before and after redemption. A concurrent login
      // owns its new identity; recovery never signs out an unrelated account.
      await client.cancelQueries({ queryKey: authKeys.session })
      if (!alive.current) return
      await mutation.mutateAsync(() => passwordRecoveryApi.redeem(grantId, secret, supplied, confirmation))
      token.current = null
      if (alive.current) setCaptured(null)
      if (alive.current) { setPassword(''); setRepeat(''); setSuccess(true) }
      client.removeQueries({ queryKey: previewKey, exact: true })
      try { if (sameRecoverySession(client, initial)) await refreshRecoverySession(client, initial) }
      catch { if (alive.current) setFeedback('Passordet er lagret, men økten kunne ikke oppdateres. Oppdater siden før du fortsetter.') }
    } catch (error) {
      if (!alive.current) return
      setFeedback(recoveryMessage(error))
      setPassword(''); setRepeat('')
      if (error instanceof ApiHttpError && error.code === 'password_recovery_invalid') { setInvalid(true); token.current = null; setCaptured(null) }
    } finally { mutation.reset(); busy.current = false; if (alive.current) setPending(false) }
  }
  const unavailable = !valid || invalid || (preview.error instanceof ApiHttpError && preview.error.code === 'password_recovery_invalid')
  return <section className="sign-in-panel" aria-labelledby="reset-heading">
    <p className="brand">Guttas Golf</p><h1 id="reset-heading">Velg nytt passord</h1>
    {success ? <>
      <p role="status">Passordet er endret. Kontoens tidligere økter er avsluttet. Logg inn med det nye passordet.</p>
      {feedback && <p role="alert">{feedback}</p>}
      <Link to="/login" state={{ passwordRecoveryComplete: true }}>Gå til innlogging</Link>
    </> : unavailable ? <><p role="alert">{invalidRecoveryMessage}</p><Link to="/login">Til innlogging</Link></> : <>
      {preview.isPending && <p role="status">Kontrollerer lenken …</p>}
      {preview.error && <><p role="alert">{recoveryMessage(preview.error)}</p><button type="button" disabled={preview.isFetching} onClick={() => void preview.refetch()}>Prøv igjen</button></>}
      {preview.data && !preview.error && <>
        <p>Lenken kan brukes én gang og utløper {recoveryExpiry(preview.data.expires_at)}.</p>
        <form onSubmit={(event) => void submit(event)} aria-busy={pending}>
          <p id="reset-password-help">{PASSWORD_GUIDANCE}</p>
          <label><span>Nytt passord</span><input type="password" autoComplete="new-password" aria-describedby="reset-password-help" required value={password} disabled={pending} onChange={(event) => setPassword(event.target.value)} /></label>
          <label><span>Gjenta nytt passord</span><input type="password" autoComplete="new-password" required value={repeat} disabled={pending} onChange={(event) => setRepeat(event.target.value)} /></label>
          {feedback && <p role="alert" className="sign-in-error">{feedback}</p>}
          <button type="submit" disabled={pending}>{pending ? 'Lagrer passord …' : 'Lagre nytt passord'}</button>
        </form>
      </>}
    </>}
  </section>
}
