import { useState } from 'react'
import type { AuthSession } from '../../api/auth'
import type { Profile } from '../../api/profile'
import { profileApi } from '../../api/profileRequests'
import { USERNAME_HTML_PATTERN } from '../auth/username'

interface Props {
  kind: 'username' | 'password'
  profile: Profile
  session: AuthSession
  disabled: boolean
  run: (operation: () => Promise<unknown>, password?: boolean) => Promise<boolean>
}
export function ProfileCredentialsForm({ kind, profile, session, disabled, run }: Props) {
  const [username, setUsername] = useState(profile.username)
  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmation, setConfirmation] = useState('')
  const [error, setError] = useState<string | null>(null)
  const password = kind === 'password'
  return <form className="profile-form" onSubmit={(event) => {
    event.preventDefault()
    setError(null)
    if (password && newPassword !== confirmation) { setError('De nye passordene er ikke like.'); return }
    if (password && (new TextEncoder().encode(newPassword).length < 12 || new TextEncoder().encode(newPassword).length > 128)) {
      setError('Passordet må være mellom 12 og 128 byte. Bokstaver som æ, ø og å bruker flere byte.'); return
    }
    void run(() => password ? profileApi.password(profile.version, newPassword, currentPassword, session.csrf_token)
      : profileApi.username(profile.version, username.trim(), currentPassword, session.csrf_token), password)
      .finally(() => { setCurrentPassword(''); setNewPassword(''); setConfirmation('') })
  }}>
    <h2>{password ? 'Bytt passord' : 'Endre brukernavn'}</h2>
    <fieldset disabled={disabled}>
      {password ? <>
        <p>Passordbytte logger deg ut på alle enheter. Logg inn igjen med det nye passordet.</p>
        <label>Nytt passord<input type="password" autoComplete="new-password" maxLength={128} required value={newPassword} onChange={(event) => setNewPassword(event.target.value)} /></label>
        <label>Gjenta nytt passord<input type="password" autoComplete="new-password" maxLength={128} required value={confirmation} onChange={(event) => setConfirmation(event.target.value)} /></label>
        <p className="muted">12–128 byte. Vanlige bokstaver og tall bruker én byte; æ, ø og å bruker flere.</p>
      </> : <>
        <label>Brukernavn<input name="username" autoComplete="username" required minLength={3} maxLength={32} pattern={USERNAME_HTML_PATTERN} value={username} onChange={(event) => setUsername(event.target.value)} /></label>
        <p className="muted">3–32 bokstaver (a–z), tall, bindestrek eller understrek. Bruk det nye brukernavnet neste gang du logger inn.</p>
      </>}
      <label>Nåværende passord<input type="password" autoComplete="current-password" required maxLength={128} value={currentPassword} onChange={(event) => setCurrentPassword(event.target.value)} /></label>
      {error && <p role="alert">{error}</p>}
      <button className="button primary" type="submit">{password ? 'Bytt passord og logg ut' : 'Lagre brukernavn'}</button>
    </fieldset>
  </form>
}
