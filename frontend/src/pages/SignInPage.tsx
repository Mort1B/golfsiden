import { useState, type FormEvent } from 'react'
import { Navigate, useLocation, useNavigate, useSearchParams } from 'react-router-dom'
import { LogIn } from 'lucide-react'
import { useAuth } from '../features/auth/authContext'
import { safeReturnTo } from '../features/auth/navigation'
import '../features/recovery/recovery.css'
import { USERNAME_HTML_PATTERN } from '../features/auth/username'

export function SignInPage() {
  const auth = useAuth()
  const location = useLocation()
  const state: unknown = location.state
  const passwordChanged = typeof state === 'object' && state !== null && 'profilePasswordChanged' in state && state.profilePasswordChanged === true
  const recoveryComplete = typeof state === 'object' && state !== null && 'passwordRecoveryComplete' in state && state.passwordRecoveryComplete === true
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const returnTo = safeReturnTo(params.get('returnTo'))

  if (auth.session && !passwordChanged && !recoveryComplete) return <Navigate replace to={returnTo} />

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setSubmitting(true)
    setError(null)
    try {
      await auth.signIn(username, password)
      navigate(returnTo, { replace: true })
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : 'Innlogging mislyktes')
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <main className="sign-in-page">
      <section className="sign-in-panel" aria-labelledby="sign-in-heading">
        <p className="brand">Guttas Golf</p>
        <h1 id="sign-in-heading">Logg inn</h1>
        {passwordChanged && <p role="status">Passordet er endret. Du er logget ut på alle enheter. Logg inn med det nye passordet.</p>}
        {recoveryComplete && <p role="status">Passordet er endret. Logg inn på den gjenopprettede kontoen med det nye passordet.</p>}
        <form onSubmit={(event) => void submit(event)}>
          <label>
            <span>Brukernavn</span>
            <input autoComplete="username" minLength={3} maxLength={32} pattern={USERNAME_HTML_PATTERN} required value={username} onChange={(event) => setUsername(event.target.value)} />
          </label>
          <label><span>Passord</span><input type="password" autoComplete="current-password" required value={password} onChange={(event) => setPassword(event.target.value)} /></label>
          {error && <p className="sign-in-error" role="alert">{error}</p>}
          <button type="submit" disabled={submitting}>
            <LogIn aria-hidden="true" />
            {submitting ? 'Logger inn...' : 'Logg inn'}
          </button>
        </form>
        <details className="forgot-password"><summary>Glemt passord?</summary>
          <p>Kontakt en administrator for turneringen din gjennom en kjent kontaktkanal. Etter å ha bekreftet identiteten din kan arrangøren lage en privat lenke for nytt passord.</p>
          <p>Er du administrator, eller mangler du en arrangør som kan hjelpe? Kontakt nettstedsoperatøren. Vi sender ikke e-post for passordbytte.</p>
        </details>
      </section>
    </main>
  )
}
