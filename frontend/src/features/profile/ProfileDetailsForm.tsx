import { useState } from 'react'
import type { Profile } from '../../api/profile'
import { profileApi } from '../../api/profileRequests'
import type { AuthSession } from '../../api/auth'
import { formatHandicap, parseHandicap } from '../handicap/format'

interface Props {
  profile: Profile
  session: AuthSession
  disabled: boolean
  run: (operation: () => Promise<unknown>) => Promise<boolean>
}
export function ProfileDetailsForm({ profile, session, disabled, run }: Props) {
  const [name, setName] = useState(profile.display_name)
  const [handicap, setHandicap] = useState(profile.handicap === null ? '' : formatHandicap(profile.handicap))
  const [reason, setReason] = useState('')
  const [error, setError] = useState<string | null>(null)
  return <form className="profile-form" onSubmit={(event) => {
    event.preventDefault()
    setError(null)
    const parsed = handicap.trim() === '' && profile.player_id === null ? { ok: true as const, value: null } : parseHandicap(handicap)
    if (!parsed.ok) { setError(parsed.message); return }
    if (parsed.value !== profile.handicap && !reason.trim()) { setError('Skriv en begrunnelse for handicapendringen.'); return }
    void run(() => profileApi.details(session.user_id, { version: profile.version, player_updated_at: profile.player_updated_at,
      display_name: name.trim(), handicap: parsed.value, reason: reason.trim() }, session.csrf_token))
  }}>
    <h2>Navn og handicap</h2>
    <fieldset disabled={disabled}>
      <label>Navn<input name="display_name" autoComplete="name" required maxLength={100} value={name} onChange={(event) => setName(event.target.value)} /></label>
      <label>Handicap<input name="handicap" inputMode="decimal" disabled={profile.player_active === false} value={handicap} onChange={(event) => setHandicap(event.target.value)} aria-describedby="profile-handicap-help" /></label>
      <p id="profile-handicap-help" className="muted">Profilhandicap brukes ved påmelding til nye turneringer. Handicap og resultater i turneringer du allerede er med i, endres ikke. Bruk komma eller punktum og maks én desimal.</p>
      {profile.player_id === null && <p>Du har ingen spillerprofil ennå. Fyll inn handicap for å opprette din egen. Dette melder deg ikke på tidligere turneringer.</p>}
      {profile.player_active === false && <p>Spillerprofilen er deaktivert. Kontakt en administrator for å endre handicap.</p>}
      <label>Begrunnelse for handicapendring<input name="reason" maxLength={500} value={reason} disabled={profile.player_active === false} onChange={(event) => setReason(event.target.value)} /></label>
      <p className="muted">Begrunnelsen og hvem som gjorde endringen, lagres i handicaphistorikken. Navneendringer vises også i tidligere resultater.</p>
      {error && <p role="alert">{error}</p>}
      <button className="button primary" type="submit">Lagre navn og handicap</button>
    </fieldset>
  </form>
}
