import { useState, type FormEvent } from 'react'
import { Navigate, useNavigate, useSearchParams } from 'react-router-dom'
import { LogIn } from 'lucide-react'
import { useAuth } from '../features/auth/authContext'
import { safeReturnTo } from '../features/auth/navigation'

export function SignInPage() {
  const auth = useAuth()
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const returnTo = safeReturnTo(params.get('returnTo'))

  if (auth.session) return <Navigate replace to={returnTo} />

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
        <h1 id="sign-in-heading">Logg inn for scoring</h1>
        <form onSubmit={(event) => void submit(event)}>
          <label>
            <span>Brukernavn</span>
            <input autoComplete="username" minLength={3} maxLength={32} pattern="[A-Za-z0-9_-]{3,32}" required value={username} onChange={(event) => setUsername(event.target.value)} />
          </label>
          <label><span>Passord</span><input type="password" autoComplete="current-password" required value={password} onChange={(event) => setPassword(event.target.value)} /></label>
          {error && <p className="sign-in-error" role="alert">{error}</p>}
          <button type="submit" disabled={submitting}>
            <LogIn aria-hidden="true" />
            {submitting ? 'Logger inn...' : 'Logg inn'}
          </button>
        </form>
      </section>
    </main>
  )
}
