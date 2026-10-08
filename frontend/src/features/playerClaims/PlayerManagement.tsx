import { useState, type FormEvent } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import type { AuthSession } from '../../api/auth'
import { playerAccountKey, playerClaimsApi } from '../../api/playerClaims'
import type { Tournament, TournamentPlayerRoster } from '../../api/types'
import { EmptyState, ErrorState, LoadingState } from '../../ui/AsyncState'
import { useAuth } from '../auth/authContext'
import { formatHandicap, parseHandicap } from '../handicap/format'
import { ClaimLink } from './ClaimLink'
import { usePlayerManagement } from './usePlayerManagement'
import './playerClaims.css'
interface Props { tournament: Tournament; roster: { data: TournamentPlayerRoster | undefined; pending: boolean; error: Error | null; retry: () => void }; authorityRefreshing: boolean }
export function PlayerManagement(props: Props) {
  const { session } = useAuth()
  return session && <Management key={`${session.user_id}:${session.csrf_token}:${props.tournament.id}`} {...props} session={session} />
}
function Management({ tournament, roster, authorityRefreshing, session }: Props & { session: AuthSession }) {
  const accounts = useQuery({ queryKey: playerAccountKey(session.user_id, tournament.id), queryFn: ({ signal }) => playerClaimsApi.accounts(tournament.id, signal), retry: false })
  const actions = usePlayerManagement(tournament.id, session)
  const [name, setName] = useState('')
  const [handicap, setHandicap] = useState('')
  const [validation, setValidation] = useState<string | null>(null)
  const [remove, setRemove] = useState<string | null>(null)
  const closed = tournament.status === 'completed' || tournament.status === 'archived'
  const unavailable = authorityRefreshing || roster.pending || !!roster.error || accounts.isFetching || accounts.isPending || !!accounts.error || accounts.fetchStatus === 'paused'
  const disabled = actions.pending || unavailable
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (disabled || closed || actions.uncertain) return
    const parsed = parseHandicap(handicap)
    if (!parsed.ok) { setValidation(parsed.message); return }
    if (!name.trim() || new TextEncoder().encode(name.trim()).length > 120) { setValidation('Skriv et navn. Hvis navnet er for langt, forkort det og prøv igjen.'); return }
    setValidation(null)
    if (await actions.run('create', '', name.trim(), parsed.value)) { setName(''); setHandicap('') }
  }
  return <div className="player-management">
    <p>Opprett spilleren med navn og handicap. Del den personlige lenken slik at spilleren kan velge brukernavn og passord. Lenken varer i sju dager.</p>
    {closed && <p>Turneringen er avsluttet eller arkivert. Spillerlisten kan ikke endres; aktive spillere kan fortsatt ta i bruk kontoen sin.</p>}
    <form onSubmit={event => void submit(event)} aria-busy={actions.pending}>
      <label><span>Spillerens navn</span><input autoComplete="off" required maxLength={120} value={name} disabled={disabled || closed} onChange={event => setName(event.target.value)} /></label>
      <label><span>Spillerens handicap</span><input inputMode="decimal" required value={handicap} disabled={disabled || closed} onChange={event => setHandicap(event.target.value)} placeholder="14,4" /></label>
      {validation && <p role="alert">{validation}</p>}
      <button disabled={disabled || closed || actions.uncertain} type="submit">Opprett spiller og kontolenke</button>
    </form>
    {actions.pending && <p role="status">Lagrer og oppdaterer spillerlisten …</p>}
    {actions.feedback && <div role={actions.feedback.error ? 'alert' : 'status'}><p>{actions.feedback.text}</p>
      {actions.feedback.code === 'player_assigned_draft' && <Link to={`/manage/tournaments/${tournament.id}#pairings`}>Åpne spillegrupper, lag og matcher</Link>}
      {actions.feedback.code === 'player_round_in_progress' && <Link to={`/manage/tournaments/${tournament.id}#lifecycle`}>Åpne rundestyring</Link>}
    </div>}
    {actions.uncertain && <button disabled={disabled} onClick={actions.acknowledge}>Jeg har kontrollert listen</button>}
    {actions.receipt && <ClaimLink key={actions.receipt.claim_id} receipt={actions.receipt} current={actions.current} onHide={actions.hide} />}
    {roster.pending && <LoadingState />}
    {roster.error && <ErrorState error={roster.error} onRetry={roster.retry} />}
    {accounts.isPending && <p role="status">Laster kontostatus …</p>}
    {accounts.error && <ErrorState error={new Error('Kunne ikke laste kontostatus.')} onRetry={() => void accounts.refetch()} />}
    {roster.data?.players.length === 0 && <EmptyState>Ingen deltakere er registrert.</EmptyState>}
    {roster.data && <ul className="managed-players">{roster.data.players.map(player => {
      const account = accounts.data?.find(item => item.player_id === player.player_id)
      const withdrawn = player.status === 'withdrawn'
      return <li key={player.player_id}>
        <strong>{player.display_name}</strong><span>HCP {formatHandicap(player.tournament_handicap)} · {withdrawn ? 'Trukket' : account?.has_account ? 'Konto tatt i bruk' : 'Venter på at spilleren tar i bruk kontoen'}</span>
        {!withdrawn && <div className="claim-actions">
          {account && !account.has_account && <><button disabled={disabled} onClick={() => void actions.run('reissue', player.player_id)} aria-label={`Lag ny kontolenke for ${player.display_name}`}>Lag ny kontolenke</button>
            <button disabled={disabled || !account.claim_id || !!account.claim_revoked_at} onClick={() => void actions.run('revoke', player.player_id)} aria-label={`Tilbakekall kontolenke for ${player.display_name}`}>Tilbakekall kontolenke</button></>}
          {player.player_id !== session.player_id && <button disabled={disabled || closed} onClick={() => setRemove(player.player_id)} aria-label={`Fjern ${player.display_name} fra turneringen`}>Fjern fra turneringen</button>}
        </div>}
        {remove === player.player_id && !withdrawn && <div className="claim-confirmation">
          <p>Fjerne <strong>{player.display_name}</strong> fra aktiv deltakelse? Historiske resultater, handicap og konto beholdes. Eksisterende tilgang og roller beholdes. Kontolenker blir ugyldige.</p>
          <div className="claim-actions"><button disabled={disabled || closed} onClick={() => void actions.run('withdraw', player.player_id).then(ok => { if (ok) setRemove(null) })}>Bekreft fjerning</button><button disabled={actions.pending} onClick={() => setRemove(null)}>Avbryt</button></div>
        </div>}
      </li>
    })}</ul>}
    <Link to={`/tournaments/${tournament.id}`}>Til spillerlisten med handicap og passordhjelp</Link>
  </div>
}
