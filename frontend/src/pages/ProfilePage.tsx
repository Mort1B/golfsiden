import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { authKeys, type AuthSession } from '../api/auth'
import { api } from '../api/client'
import { profileKeys } from '../api/profile'
import { profileApi } from '../api/profileRequests'
import { tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'
import { ProfileDetailsForm } from '../features/profile/ProfileDetailsForm'
import { ProfileCredentialsForm } from '../features/profile/ProfileCredentialsForm'
import { useProfileActions } from '../features/profile/useProfileActions'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { StatusBadge } from '../ui/StatusBadge'
import '../features/profile/profile.css'

export function ProfilePage() {
  const { session } = useAuth()
  return session ? <ProfileWorkspace key={`${session.user_id}:${session.csrf_token}`} session={session} /> : null
}
function ProfileWorkspace({ session }: { session: AuthSession }) {
  const client = useQueryClient()
  const sameSession = () => {
    const current = client.getQueryData<AuthSession | null>(authKeys.session)
    return current?.user_id === session.user_id && current.csrf_token === session.csrf_token
  }
  const readForSession = async <T,>(request: () => Promise<T>): Promise<T> => {
    if (!sameSession()) throw new Error('Økten er endret. Logg inn igjen.')
    const value = await request()
    if (!sameSession()) throw new Error('Økten er endret. Logg inn igjen.')
    return value
  }
  const profile = useQuery({ queryKey: profileKeys.me(session.user_id), queryFn: () => readForSession(() => profileApi.get(session.user_id)), enabled: sameSession(), retry: false })
  const tournaments = useQuery({ queryKey: tournamentKeys.mine(session.user_id), queryFn: () => readForSession(api.myTournaments), enabled: sameSession(), retry: false })
  const actions = useProfileActions(session)
  const data = profile.error ? undefined : profile.data
  const disabled = actions.pending || profile.isFetching
  return <section className="page profile-page">
    <header className="page-header"><p className="brand">Guttas Golf</p><h1>Min profil</h1><p>Kontoinnstillinger og turneringene dine.</p></header>
    <button className="button secondary" type="button" disabled={disabled} onClick={() => { void profile.refetch(); void tournaments.refetch() }}>Oppdater profil og turneringer</button>
    <p className="muted">Oppdatering henter lagrede opplysninger. Endringer som ikke er lagret, kan bli erstattet.</p>
    {profile.isPending && <LoadingState />}
    {profile.error && <ErrorState error={profile.error} onRetry={() => void profile.refetch()} />}
    {profile.isFetching && !profile.isPending && <p role="status">Oppdaterer profil …</p>}
    {actions.pending && <p role="status">Lagrer og kontrollerer endringen …</p>}
    {actions.feedback && <p role={actions.feedback.error ? 'alert' : 'status'}>{actions.feedback.text}</p>}
    {data && <div key={`${data.version}-${data.player_updated_at}`}>
      <ProfileDetailsForm profile={data} session={session} disabled={disabled} run={actions.run} />
      <ProfileCredentialsForm kind="username" profile={data} session={session} disabled={disabled} run={actions.run} />
      <ProfileCredentialsForm kind="password" profile={data} session={session} disabled={disabled} run={actions.run} />
    </div>}
    <section aria-labelledby="profile-tournaments"><h2 id="profile-tournaments">Mine turneringer</h2>
      <Link className="button primary" to="/create">Opprett ny turnering</Link>
      {tournaments.isPending && <LoadingState />}
      {tournaments.error ? <ErrorState error={tournaments.error} onRetry={() => void tournaments.refetch()} /> : <>
        {tournaments.data?.length === 0 && <EmptyState>Du er ikke med i noen turneringer ennå.</EmptyState>}
        <div className="item-list">{tournaments.data?.map(({ tournament, role }) => <Link className="list-card" to={`/tournaments/${tournament.id}`} key={tournament.id}>
          <StatusBadge status={tournament.status} /><h3>{tournament.name}</h3>
          <p>{role === 'admin' ? 'Administrator' : role === 'scorer' ? 'Scorefører' : role === 'player' ? 'Spiller' : 'Tilskuer'}</p>
        </Link>)}</div>
      </>}
      <Link to="/tournaments">Gå til turneringsoversikten</Link>
    </section>
  </section>
}
