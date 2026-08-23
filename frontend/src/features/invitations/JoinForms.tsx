import { useState, type FormEvent } from 'react'
import { LogIn, UserPlus } from 'lucide-react'
import type { InvitationRegistrationInput } from '../../api/invitations'
import { USERNAME_HTML_PATTERN } from '../auth/username'
import { parseHandicap } from '../handicap/format'

interface RegistrationFormProps {
  disabled: boolean
  error: string | null
  onSubmit: (input: InvitationRegistrationInput) => Promise<void>
}

export function RegistrationForm({ disabled, error, onSubmit }: RegistrationFormProps) {
  const [displayName, setDisplayName] = useState('')
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [handicap, setHandicap] = useState('')
  const [validationError, setValidationError] = useState<string | null>(null)

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const parsedHandicap = parseHandicap(handicap)
    if (!parsedHandicap.ok) {
      setValidationError(parsedHandicap.message)
      return
    }
    setValidationError(null)
    await onSubmit({
      account: { username: username.trim().toLowerCase(), password },
      player: { display_name: displayName, handicap_index: parsedHandicap.value },
    })
  }

  return (
    <section className="join-section" aria-labelledby="registration-heading">
      <h2 id="registration-heading">Ny spiller</h2>
      <form className="invitation-form" onSubmit={(event) => void submit(event)}>
        <label><span>Visningsnavn</span><input autoComplete="name" required maxLength={120} value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></label>
        <label><span>Brukernavn</span><input autoComplete="username" minLength={3} maxLength={32} pattern={USERNAME_HTML_PATTERN} required aria-describedby="join-username-help" value={username} onChange={(event) => setUsername(event.target.value)} /><small id="join-username-help">3–32 bokstaver, tall, bindestrek eller understrek.</small></label>
        <label><span>Passord</span><input type="password" autoComplete="new-password" minLength={12} maxLength={128} required aria-describedby="join-password-help" value={password} onChange={(event) => setPassword(event.target.value)} /><small id="join-password-help">Minst 12 tegn.</small></label>
        <label><span>Handicapindeks</span><input type="text" inputMode="decimal" required aria-describedby="join-handicap-help" value={handicap} onChange={(event) => setHandicap(event.target.value)} /><small id="join-handicap-help">Bruk komma eller punktum, for eksempel 14,4.</small></label>
        {(validationError || error) && <p className="invitation-error" role="alert">{validationError ?? error}</p>}
        <button className="invitation-primary" type="submit" disabled={disabled}><UserPlus aria-hidden="true" />{disabled ? 'Melder på …' : 'Opprett konto og bli med'}</button>
      </form>
    </section>
  )
}

interface InlineSignInProps {
  disabled: boolean
  error: string | null
  onSubmit: (username: string, password: string) => Promise<void>
}

export function InlineSignIn({ disabled, error, onSubmit }: InlineSignInProps) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    await onSubmit(username, password)
  }

  return (
    <section className="join-section existing-account" aria-labelledby="existing-account-heading">
      <h2 id="existing-account-heading">Har du allerede konto?</h2>
      <form className="invitation-form" onSubmit={(event) => void submit(event)}>
        <label><span>Brukernavn</span><input autoComplete="username" minLength={3} maxLength={32} pattern={USERNAME_HTML_PATTERN} required value={username} onChange={(event) => setUsername(event.target.value)} /></label>
        <label><span>Passord</span><input type="password" autoComplete="current-password" required value={password} onChange={(event) => setPassword(event.target.value)} /></label>
        {error && <p className="invitation-error" role="alert">{error}</p>}
        <button className="invitation-secondary" type="submit" disabled={disabled}><LogIn aria-hidden="true" />{disabled ? 'Logger inn …' : 'Logg inn'}</button>
      </form>
    </section>
  )
}
