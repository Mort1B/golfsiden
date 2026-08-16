import { Check, Clipboard, ExternalLink } from 'lucide-react'
import { useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { buildInvitationUrl } from '../../api/onboarding'
import type { OnboardingSuccess } from './useOnboardingSubmission'

function formatExpiry(timestamp: string): string {
  return new Intl.DateTimeFormat('nb-NO', { dateStyle: 'long', timeStyle: 'short' }).format(new Date(timestamp))
}

export function OnboardingSuccessView({ success }: { success: OnboardingSuccess }) {
  const [copyStatus, setCopyStatus] = useState<string | null>(null)
  const invitationField = useRef<HTMLInputElement>(null)
  const invitationUrl = buildInvitationUrl(window.location.origin, success.invitation.id, success.invitation.token)

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(invitationUrl)
      setCopyStatus('Invitasjonslenken er kopiert.')
    } catch {
      setCopyStatus('Kunne ikke kopiere automatisk. Marker og kopier lenken manuelt.')
      invitationField.current?.focus()
      invitationField.current?.select()
    }
  }

  return (
    <main className="onboarding-page success-page">
      <section className="success-panel" aria-labelledby="success-heading">
        <div className="success-mark" aria-hidden="true"><Check /></div>
        <p className="eyebrow">Turneringen er opprettet</p>
        <h1 id="success-heading">{success.tournament.name}</h1>
        <p>{success.rounds.length} {success.rounds.length === 1 ? 'runde' : 'runder'} er lagret, og du er administrator og spiller.</p>

        <div className="invitation-block">
          <h2>Invitasjonslenke</h2>
          <p>Ta vare på lenken; den vises bare én gang.</p>
          <label htmlFor="invitation-link">Lenke til deltakerne</label>
          <input id="invitation-link" ref={invitationField} readOnly value={invitationUrl} onFocus={(event) => event.currentTarget.select()} onClick={(event) => event.currentTarget.select()} />
          <button className="button secondary copy-button" type="button" onClick={() => void copy()}>
            <Clipboard aria-hidden="true" /> Kopier lenke
          </button>
          <p className="copy-status" role="status">{copyStatus ?? `Utløper ${formatExpiry(success.invitation.expiresAt)}.`}</p>
        </div>

        <Link className="button primary open-tournament" to={`/tournaments/${success.tournament.id}`}>
          Åpne turneringen <ExternalLink aria-hidden="true" />
        </Link>
      </section>
    </main>
  )
}
