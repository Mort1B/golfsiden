import { useState } from 'react'
import type { ResultShareReceipt as Receipt } from '../../api/resultSharing'
import { sharedResultsUrl, shareExpiry } from './format'

export function ResultShareReceipt({ receipt, hide }: { receipt: Receipt; hide: () => void }) {
  const [message, setMessage] = useState('')
  const url = sharedResultsUrl(receipt.grant.id, receipt.token)
  const copy = async () => {
    try { await navigator.clipboard.writeText(url); setMessage('Resultatlenken er kopiert.') }
    catch { setMessage('Kunne ikke kopiere. Marker lenken og kopier manuelt.') }
  }
  return <div className="result-share-receipt">
    <p>Lenken vises bare nå. Den kan brukes flere ganger frem til {shareExpiry(receipt.grant.expires_at)}.</p>
    <label><span>Offentlig resultatlenke</span><input readOnly autoComplete="off" value={url} onFocus={event => event.currentTarget.select()} /></label>
    <div className="result-share-actions"><button type="button" onClick={() => void copy()}>Kopier resultatlenke</button><button type="button" onClick={hide}>Skjul resultatlenken</button></div>
    <p aria-live="polite">{message}</p>
  </div>
}
