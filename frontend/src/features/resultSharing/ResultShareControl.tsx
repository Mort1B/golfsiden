import { useEffect, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { ApiHttpError } from '../../api/http'
import { resultSharingApi, resultShareKey, type ResultShareReceipt as Receipt } from '../../api/resultSharing'
import { useAuth } from '../auth/authContext'
import { ResultShareReceipt } from './ResultShareReceipt'
import { shareExpiry } from './format'
import './resultSharing.css'

interface Props { tournamentId: string; authorityRefreshing: boolean }
export function ResultShareControl(props: Props) {
  const auth = useAuth()
  if (!auth.session || auth.error) return null
  return <Control key={`${auth.session.user_id}:${auth.session.csrf_token}:${props.tournamentId}`} {...props} session={auth.session} />
}
function Control({ tournamentId, authorityRefreshing, session }: Props & { session: AuthSession }) {
  const client = useQueryClient()
  const query = useQuery({ queryKey: resultShareKey(session.user_id, tournamentId), queryFn: () => resultSharingApi.status(tournamentId), retry: false })
  const alive = useRef(false); const submitting = useRef(false)
  const [pending, setPending] = useState(false)
  const [receipt, setReceipt] = useState<Receipt | null>(null)
  const [message, setMessage] = useState<{ error: boolean; text: string } | null>(null)
  const [now, setNow] = useState(Date.now)
  useEffect(() => {
    alive.current = true
    const timer = setInterval(() => setNow(Date.now()), 60_000)
    return () => { alive.current = false; clearInterval(timer) }
  }, [])
  const mutation = useMutation({ mutationFn: (operation: () => Promise<void>) => operation(), gcTime: 0, retry: false })
  const current = () => {
    const active = client.getQueryData<AuthSession | null>(authKeys.session)
    return alive.current && active?.user_id === session.user_id && active.csrf_token === session.csrf_token
  }
  const disabled = authorityRefreshing || pending || query.isFetching || !!query.error || !query.data
  const grant = query.data?.grant
  const expired = grant !== undefined && grant !== null && Date.parse(grant.expires_at) <= now
  const refresh = async () => { setReceipt(null); await query.refetch() }
  const run = async (revoke: boolean) => {
    if (submitting.current || disabled || !current() || (revoke && !grant)) return
    submitting.current = true; setPending(true); setReceipt(null); setMessage(null)
    let issued: Receipt | null = null
    try {
      await mutation.mutateAsync(async () => {
        if (revoke && grant) await resultSharingApi.revoke(tournamentId, grant.id, session.csrf_token)
        else issued = await resultSharingApi.issue(tournamentId, grant?.id ?? null, session.csrf_token)
      })
      if (!current()) return
      const fresh = await query.refetch()
      if (!current()) return
      if (fresh.error) throw new Error('refresh')
      setReceipt(issued)
      setMessage({ error: false, text: revoke ? 'Resultatlenken er tilbakekalt.' : 'Resultatlenken er opprettet.' })
    } catch (error) {
      if (!current()) return
      setReceipt(null)
      setMessage({ error: true, text: error instanceof ApiHttpError && error.code === 'result_share_stale'
        ? 'Lenken ble endret et annet sted. Kontroller oppdatert status før du prøver igjen.'
        : 'Handlingen kunne ikke bekreftes. Oppdater status og prøv igjen.' })
      await query.refetch()
    } finally { mutation.reset(); submitting.current = false; if (current()) setPending(false) }
  }
  const visibleReceipt = !query.error && !query.isFetching && !authorityRefreshing && receipt?.grant.id === grant?.id && !grant?.revoked_at && !expired ? receipt : null
  return <section className="result-share-control" aria-labelledby="result-sharing-heading" aria-busy={pending}>
    <h3 id="result-sharing-heading">Del resultater offentlig</h3>
    <p>Alle med lenken kan se turneringsnavnet, spillernes navn og sammenlagtresultater i brutto og netto uten å logge inn. Skjulte finaleresultater forblir skjult.</p>
    <p>Lenken varer i 30 dager. Du kan erstatte eller tilbakekalle den når som helst. Resultater som allerede er lest kan ikke trekkes tilbake; en åpen side oppdateres hvert 15. sekund.</p>
    {query.isPending && <p role="status">Henter status for resultatdeling …</p>}
    {query.error && <div role="alert"><p>Kunne ikke hente status for resultatdeling.</p><button type="button" onClick={() => void refresh()}>Oppdater delingsstatus</button></div>}
    {!query.error && query.data && <>
      <p className="result-share-status">{!grant ? 'Ingen resultatlenke er opprettet.' : grant.revoked_at ? 'Den siste resultatlenken er tilbakekalt.' : expired ? 'Den siste resultatlenken er utløpt.' : `Resultatdeling er aktiv til ${shareExpiry(grant.expires_at)}.`}</p>
      {authorityRefreshing && <p role="status">Kontrollerer administratortilgang …</p>}
      <div className="result-share-actions">
        <button type="button" disabled={disabled} onClick={() => void run(false)}>{grant ? 'Erstatt resultatlenken' : 'Opprett offentlig resultatlenke'}</button>
        {grant && !grant.revoked_at && !expired && <button type="button" disabled={disabled} onClick={() => void run(true)}>Tilbakekall resultatlenken</button>}
      </div>
      {grant && <p>En erstatningslenke gjør den gamle lenken ugyldig med én gang.</p>}
    </>}
    {pending && <p role="status">Oppdaterer resultatdeling …</p>}
    {message && <p role={message.error ? 'alert' : 'status'}>{message.text}</p>}
    {visibleReceipt && <ResultShareReceipt key={visibleReceipt.grant.id} receipt={visibleReceipt} hide={() => setReceipt(null)} />}
  </section>
}
