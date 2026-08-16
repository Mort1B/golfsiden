import { useState, type FormEvent } from 'react'
import { LogIn, UserPlus } from 'lucide-react'
import type { InvitationRegistrationInput } from '../../api/invitations'

interface RegistrationFormProps {
  disabled: boolean
  error: string | null
  onSubmit: (input: InvitationRegistrationInput) => Promise<void>
}

export function RegistrationForm({ disabled, error, onSubmit }: RegistrationFormProps) {
  const [displayName, setDisplayName] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [handicap, setHandicap] = useState('')
  const [validationError, setValidationError] = useState<string | null>(null)

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const handicapIndex = Number(handicap)
    if (!handicap.trim() || !Number.isFinite(handicapIndex) || handicapIndex < -10 || handicapIndex > 54) {
      setValidationError('Handicap må være et tall mellom −10,0 og 54,0.')
      return
    }
    setValidationError(null)
    await onSubmit({
      account: { email, password },
      player: { display_name: displayName, handicap_index: handicapIndex },
    })
  }

  return (
    <section className="join-section" aria-labelledby="registration-heading">
      <h2 id="registration-heading">Ny spiller</h2>
      <form className="invitation-form" onSubmit={(event) => void submit(event)}>
        <label><span>Visningsnavn</span><input autoComplete="name" required maxLength={120} value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></label>
        <label><span>E-post</span><input type="email" inputMode="email" autoComplete="username" required value={email} onChange={(event) => setEmail(event.target.value)} /></label>
        <label><span>Passord</span><input type="password" autoComplete="new-password" minLength={12} maxLength={128} required aria-describedby="join-password-help" value={password} onChange={(event) => setPassword(event.target.value)} /><small id="join-password-help">Minst 12 tegn.</small></label>
        <label><span>Handicapindeks</span><input type="number" inputMode="decimal" step="0.1" min="-10" max="54" required value={handicap} onChange={(event) => setHandicap(event.target.value)} /></label>
        {(validationError || error) && <p className="invitation-error" role="alert">{validationError ?? error}</p>}
        <button className="invitation-primary" type="submit" disabled={disabled}><UserPlus aria-hidden="true" />{disabled ? 'Melder på …' : 'Opprett konto og bli med'}</button>
      </form>
    </section>
  )
}

interface InlineSignInProps {
  disabled: boolean
  error: string | null
  onSubmit: (email: string, password: string) => Promise<void>
}

export function InlineSignIn({ disabled, error, onSubmit }: InlineSignInProps) {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    await onSubmit(email, password)
  }

  return (
    <section className="join-section existing-account" aria-labelledby="existing-account-heading">
      <h2 id="existing-account-heading">Har du allerede konto?</h2>
      <form className="invitation-form" onSubmit={(event) => void submit(event)}>
        <label><span>E-post</span><input type="email" inputMode="email" autoComplete="username" required value={email} onChange={(event) => setEmail(event.target.value)} /></label>
        <label><span>Passord</span><input type="password" autoComplete="current-password" required value={password} onChange={(event) => setPassword(event.target.value)} /></label>
        {error && <p className="invitation-error" role="alert">{error}</p>}
        <button className="invitation-secondary" type="submit" disabled={disabled}><LogIn aria-hidden="true" />{disabled ? 'Logger inn …' : 'Logg inn'}</button>
      </form>
    </section>
  )
}
