import { useState } from 'react'
import type { RecoveryReceipt } from '../../api/passwordRecovery'
import { recoveryExpiry } from './messages'

export function RecoveryLink({ receipt, onHide }: { receipt: RecoveryReceipt; onHide: () => void }) {
  const [status, setStatus] = useState('')
  const copy = async () => {
    try { await navigator.clipboard.writeText(receipt.reset_url); setStatus('Lenken er kopiert.') }
    catch { setStatus('Kunne ikke kopiere. Marker lenken og kopier manuelt.') }
  }
  return <div className="recovery-link">
    <p role="status">Lenken vises bare nå. Del den privat med spilleren du har bekreftet identiteten til.</p>
    <p>Utløper {recoveryExpiry(receipt.expires_at)}. Kan brukes én gang.</p>
    <label><span>Lenke for nytt passord</span><input readOnly autoComplete="off" value={receipt.reset_url} onFocus={(event) => event.currentTarget.select()} /></label>
    <div className="recovery-actions"><button type="button" onClick={() => void copy()}>Kopier lenke</button><button type="button" onClick={onHide}>Skjul lenken</button></div>
    <p aria-live="polite">{status}</p>
  </div>
}
