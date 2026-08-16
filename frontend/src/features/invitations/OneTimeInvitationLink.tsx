import { useState } from 'react'
import { Check, Copy, X } from 'lucide-react'

interface OneTimeInvitationLinkProps {
  url: string
  onDismiss: () => void
}

export function OneTimeInvitationLink({ url, onDismiss }: OneTimeInvitationLinkProps) {
  const [copied, setCopied] = useState(false)
  const [copyError, setCopyError] = useState<string | null>(null)

  const copy = async () => {
    setCopyError(null)
    try {
      await navigator.clipboard.writeText(url)
      setCopied(true)
    } catch {
      setCopyError('Kunne ikke kopiere. Marker lenken og kopier manuelt.')
    }
  }

  return (
    <section className="one-time-link" aria-labelledby="new-link-heading">
      <div><p className="eyebrow">Vises én gang</p><h2 id="new-link-heading">Ny invitasjonslenke</h2></div>
      <button className="invitation-icon-button dismiss-link" type="button" aria-label="Skjul invitasjonslenken" title="Skjul lenken" onClick={onDismiss}><X aria-hidden="true" /></button>
      <label><span>Del denne lenken</span><input readOnly value={url} onFocus={(event) => event.currentTarget.select()} /></label>
      <button className="invitation-secondary" type="button" onClick={() => void copy()}>{copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}{copied ? 'Kopiert' : 'Kopier lenke'}</button>
      <p className="one-time-help">Hvis lenken går tapt, roter invitasjonen for å lage en ny.</p>
      {copyError && <p className="invitation-error" role="alert">{copyError}</p>}
    </section>
  )
}
