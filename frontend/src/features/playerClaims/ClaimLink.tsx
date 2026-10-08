import { useEffect, useRef, useState } from 'react'
import type { ClaimReceipt } from '../../api/playerClaims'
import { recoveryExpiry } from '../recovery/messages'
export function ClaimLink({ receipt, current, onHide }: { receipt: ClaimReceipt; current: () => boolean; onHide: () => void }) {
  const url = new URL(`/claim/${receipt.claim_id}`, window.location.origin)
  url.hash = new URLSearchParams({ token: receipt.token }).toString()
  const [status, setStatus] = useState('')
  const alive = useRef(false)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const copy = async () => {
    try { await navigator.clipboard.writeText(url.href); if (alive.current && current()) setStatus('Lenken er kopiert.') }
    catch { if (alive.current && current()) setStatus('Kunne ikke kopiere. Marker lenken og kopier manuelt.') }
  }
  return <div className="claim-link">
    <p role="status">Spillerens personlige lenke vises bare nå. Del den privat med riktig person.</p>
    <p>Kan brukes én gang, innen {recoveryExpiry(receipt.expires_at)}. En ny lenke erstatter den gamle.</p>
    <label><span>Personlig kontolenke</span><input readOnly autoComplete="off" value={url.href} onFocus={event => event.currentTarget.select()} /></label>
    <div className="claim-actions"><button type="button" onClick={() => void copy()}>Kopier lenke</button><button type="button" onClick={onHide}>Skjul lenken</button></div>
    <p aria-live="polite">{status}</p>
  </div>
}
