import { useState, type FormEvent } from 'react'
import { Plus } from 'lucide-react'
import type { InvitationIssueInput } from '../../api/invitations'
import { parseMaximumUses, toExpiryTimestamp } from './adminState'

interface InvitationIssueFormProps {
  disabled: boolean
  error: string | null
  onIssue: (input: InvitationIssueInput) => Promise<void>
}

export function InvitationIssueForm({ disabled, error, onIssue }: InvitationIssueFormProps) {
  const [expiry, setExpiry] = useState('')
  const [maximumUses, setMaximumUses] = useState('')
  const [validationError, setValidationError] = useState<string | null>(null)

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const expiresAt = toExpiryTimestamp(expiry, Date.now())
    const maxUses = parseMaximumUses(maximumUses)
    if (!expiresAt) {
      setValidationError('Velg et utløpstidspunkt fram i tid.')
      return
    }
    if (maxUses === 'invalid') {
      setValidationError('Maks antall må være et positivt heltall eller stå tomt.')
      return
    }
    setValidationError(null)
    await onIssue({ expires_at: expiresAt, max_uses: maxUses })
  }

  return (
    <section className="invitation-admin-section" aria-labelledby="issue-invitation-heading">
      <div className="invitation-section-heading"><div><p className="eyebrow">Ny lenke</p><h2 id="issue-invitation-heading">Opprett invitasjon</h2></div></div>
      <form className="invitation-issue-form" onSubmit={(event) => void submit(event)}>
        <label><span>Utløper</span><input type="datetime-local" required value={expiry} onChange={(event) => setExpiry(event.target.value)} /></label>
        <label><span>Maks antall påmeldinger</span><input type="number" inputMode="numeric" min="1" step="1" placeholder="Ubegrenset" value={maximumUses} onChange={(event) => setMaximumUses(event.target.value)} /></label>
        {(validationError || error) && <p className="invitation-error" role="alert">{validationError ?? error}</p>}
        <button className="invitation-primary" type="submit" disabled={disabled}><Plus aria-hidden="true" />{disabled ? 'Oppretter …' : 'Opprett lenke'}</button>
      </form>
    </section>
  )
}
