import { useState } from 'react'
import { RefreshCw, Trash2 } from 'lucide-react'
import type { InvitationMetadata } from '../../api/invitations'
import { invitationStatus, type InvitationStatus } from './adminState'

interface InvitationListProps {
  invitations: InvitationMetadata[]
  pendingId: string | null
  onRotate: (invitationId: string) => Promise<void>
  onRevoke: (invitationId: string) => Promise<void>
}

const statusLabels: Record<InvitationStatus, string> = {
  active: 'Aktiv', expired: 'Utløpt', revoked: 'Tilbakekalt', exhausted: 'Brukt opp',
}

function formatTimestamp(timestamp: string): string {
  return new Intl.DateTimeFormat('nb-NO', {
    day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit',
  }).format(new Date(timestamp))
}

export function InvitationList({ invitations, pendingId, onRotate, onRevoke }: InvitationListProps) {
  const [confirmingId, setConfirmingId] = useState<string | null>(null)
  const now = Date.now()
  if (invitations.length === 0) return <div className="invitation-empty">Ingen invitasjoner er opprettet.</div>

  return (
    <ul className="invitation-list">
      {invitations.map((invitation) => {
        const status = invitationStatus(invitation, now)
        const isActive = status === 'active'
        const pending = pendingId === invitation.id
        return (
          <li key={invitation.id}>
            <div className="invitation-row-heading"><span className={`invitation-status invitation-status-${status}`}>{statusLabels[status]}</span><span>{invitation.redemption_count}{invitation.max_uses === null ? ' påmeldt' : ` av ${invitation.max_uses} påmeldt`}</span></div>
            <dl>
              <div><dt>Utløper</dt><dd>{formatTimestamp(invitation.expires_at)}</dd></div>
              <div><dt>Opprettet</dt><dd>{formatTimestamp(invitation.created_at)}</dd></div>
              {status === 'revoked' && <div><dt>Tilbakekalt</dt><dd>{invitation.revocation_actor_known ? formatTimestamp(invitation.revoked_at ?? invitation.created_at) : 'Eldre historikk uten kjent aktør'}</dd></div>}
            </dl>
            {isActive && (
              <div className="invitation-row-actions">
                <button className="invitation-secondary" type="button" disabled={pendingId !== null} onClick={() => void onRotate(invitation.id)}><RefreshCw aria-hidden="true" />{pending ? 'Roterer …' : 'Roter lenke'}</button>
                {confirmingId === invitation.id ? (
                  <div className="revoke-confirm" role="group" aria-label="Bekreft tilbakekalling">
                    <button className="invitation-danger" type="button" disabled={pendingId !== null} onClick={() => void onRevoke(invitation.id)}>Bekreft</button>
                    <button className="invitation-secondary" type="button" disabled={pendingId !== null} onClick={() => setConfirmingId(null)}>Avbryt</button>
                  </div>
                ) : (
                  <button className="invitation-secondary" type="button" disabled={pendingId !== null} onClick={() => setConfirmingId(invitation.id)}><Trash2 aria-hidden="true" />Tilbakekall</button>
                )}
              </div>
            )}
          </li>
        )
      })}
    </ul>
  )
}
