import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ChevronLeft } from 'lucide-react'
import { Link, useLocation, useParams } from 'react-router-dom'
import { isCanonicalUuid } from '../api/decoder'
import { tournamentApi, tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'
import { MANAGEMENT_SECTIONS, managementSectionFromHash, resolveManagementAccess } from '../features/tournaments/managementWorkspace'
import { TournamentManagementSections } from '../features/tournaments/TournamentManagementSections'
import { ErrorState, LoadingState } from '../ui/AsyncState'

export function TournamentManagementPage() {
  const { tournamentId = '' } = useParams()
  const location = useLocation()
  const userId = useAuth().session?.user_id ?? ''
  const canonical = isCanonicalUuid(tournamentId)
  const memberships = useQuery({
    queryKey: tournamentKeys.mine(userId),
    queryFn: tournamentApi.mine,
    enabled: canonical && userId.length > 0,
  })
  const tournament = useQuery({
    queryKey: tournamentKeys.detail(userId, tournamentId),
    queryFn: () => tournamentApi.detail(tournamentId),
    enabled: canonical && userId.length > 0,
  })
  const access = resolveManagementAccess({
    tournamentId,
    memberships: memberships.data,
    membershipsPending: memberships.isPending,
    membershipsError: memberships.error,
    tournament: tournament.data,
    tournamentPending: tournament.isPending,
    tournamentError: tournament.error,
  })
  const enabled = access.state === 'ready'
  const roster = useQuery({
    queryKey: tournamentKeys.players(userId, tournamentId),
    queryFn: () => tournamentApi.players(tournamentId),
    enabled,
  })
  const rounds = useQuery({
    queryKey: tournamentKeys.rounds(userId, tournamentId),
    queryFn: () => tournamentApi.rounds(tournamentId),
    enabled,
  })

  useEffect(() => {
    if (access.state !== 'ready') return
    const sectionId = managementSectionFromHash(location.hash)
    if (!sectionId) return
    const section = document.getElementById(sectionId)
    if (!section) return
    section.scrollIntoView({ block: 'start' })
    section.focus({ preventScroll: true })
  }, [access.state, location.hash])

  if (access.state === 'invalid') return <ManagementState title="Ugyldig turnering" message="Turneringsadressen er ikke gyldig." />
  if (access.state === 'loading') return <section className="page"><LoadingState /></section>
  if (access.state === 'missing') return <ManagementState title="Turneringen finnes ikke" message="Kontroller adressen eller gå tilbake til turneringslisten." />
  if (access.state === 'forbidden') return <ManagementState title="Ingen tilgang" message="Bare turneringsadministratorer kan åpne arbeidsområdet." />
  if (access.state === 'error') {
    return <section className="page"><ErrorState error={access.error} onRetry={() => void Promise.all([memberships.refetch(), tournament.refetch()])} /></section>
  }

  return (
    <section className="page tournament-management-page">
      <header className="detail-header management-header">
        <Link to={`/tournaments/${tournamentId}`} className="back-button" aria-label="Tilbake til turneringen"><ChevronLeft /></Link>
        <div><p className="brand">Administrasjon</p><h1>{access.tournament.name}</h1></div>
      </header>
      <nav className="management-section-nav" aria-label="Administrasjonsområder">
        {MANAGEMENT_SECTIONS.map((section) => <a key={section.id} href={`#${section.id}`}>{section.label}</a>)}
      </nav>
      <TournamentManagementSections
        tournament={access.tournament}
        roster={{ data: roster.data, pending: roster.isPending, error: roster.error, retry: () => void roster.refetch() }}
        rounds={{ data: rounds.data, pending: rounds.isPending, error: rounds.error, retry: () => void rounds.refetch() }}
      />
    </section>
  )
}

function ManagementState({ title, message }: { title: string; message: string }) {
  return <section className="page"><div className="state-message error" role="alert"><h1>{title}</h1><p>{message}</p><Link className="management-state-link" to="/tournaments">Til turneringer</Link></div></section>
}
