import { useEffect, useRef, useState, type FormEvent } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { passwordRecoveryApi, type RecoveryReceipt } from '../../api/passwordRecovery'
import { RecoveryLink } from './RecoveryLink'
import { recoveryMessage } from './messages'
import './recovery.css'

interface Props { tournamentId: string; playerId: string; playerName: string; session: AuthSession }
export function PlayerRecovery(props: Props) {
  const [open, setOpen] = useState(false)
  return <div className="player-recovery">
    <button type="button" aria-label={`${open ? 'Lukk passordhjelp' : 'Hjelp med glemt passord'} for ${props.playerName}`} aria-expanded={open} onClick={() => setOpen(!open)}>{open ? 'Lukk passordhjelp' : 'Hjelp med glemt passord'}</button>
    {open && <RecoveryForm {...props} />}
  </div>
}
function RecoveryForm({ tournamentId, playerId, playerName, session }: Props) {
  const client = useQueryClient()
  const alive = useRef(false)
  const busy = useRef(false)
  const [password, setPassword] = useState('')
  const [pending, setPending] = useState(false)
  const [feedback, setFeedback] = useState<{ error: boolean; text: string } | null>(null)
  const [receipt, setReceipt] = useState<RecoveryReceipt | null>(null)
  const mutation = useMutation({ mutationFn: (operation: () => Promise<RecoveryReceipt | void>) => operation(), gcTime: 0, retry: false })
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const current = () => {
    const cached = client.getQueryData<AuthSession | null>(authKeys.session)
    return alive.current && cached?.user_id === session.user_id && cached.csrf_token === session.csrf_token
  }
  const run = async (revoke: boolean) => {
    if (busy.current || !password || !current()) return
    busy.current = true
    setPending(true); setFeedback(null); setReceipt(null)
    const supplied = password
    setPassword('')
    try {
      const result = await mutation.mutateAsync(() => revoke
        ? passwordRecoveryApi.revoke(tournamentId, playerId, supplied, session.csrf_token)
        : passwordRecoveryApi.issue(tournamentId, playerId, supplied, session.csrf_token))
      if (!current()) return
      if (result) setReceipt(result)
      else setFeedback({ error: false, text: 'Utestående lenker for spilleren i denne turneringen er tilbakekalt.' })
    } catch (error) { if (current()) setFeedback({ error: true, text: recoveryMessage(error) }) }
    finally { mutation.reset(); busy.current = false; if (current()) setPending(false) }
  }
  const submit = (event: FormEvent<HTMLFormElement>) => { event.preventDefault(); void run(false) }
  return <div className="recovery-form">
    <p>Passordhjelp for <strong>{playerName}</strong></p>
    <p>Bekreft spillerens identitet gjennom en kjent kontaktkanal før du deler lenken. Passordet gjelder kontoen i alle turneringer.</p>
    <p>Lenken varer i 30 minutter. En ny lenke erstatter tidligere lenker. Du kan tilbakekalle lenker selv om du har lukket eller mistet dem.</p>
    <form onSubmit={submit} aria-busy={pending}>
      <label><span>Ditt nåværende passord</span><input type="password" autoComplete="current-password" required value={password} disabled={pending} onChange={(event) => setPassword(event.target.value)} /></label>
      <div className="recovery-actions">
        <button type="submit" disabled={pending || !password}>Lag lenke for nytt passord</button>
        <button type="button" disabled={pending || !password} onClick={() => void run(true)}>Tilbakekall lenker</button>
      </div>
      {pending && <p role="status">Bekrefter handlingen …</p>}
    </form>
    {feedback && <p role={feedback.error ? 'alert' : 'status'}>{feedback.text}</p>}
    {receipt && <RecoveryLink receipt={receipt} onHide={() => setReceipt(null)} />}
  </div>
}
