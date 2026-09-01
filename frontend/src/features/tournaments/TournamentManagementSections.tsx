import { ArrowRight, CalendarDays } from 'lucide-react'
import { Link } from 'react-router-dom'
import type { Round, Tournament, TournamentPlayerRoster } from '../../api/types'
import { EmptyState, ErrorState, LoadingState } from '../../ui/AsyncState'
import { StatusBadge } from '../../ui/StatusBadge'
import { CourseConfigurationSection } from './CourseConfigurationSection'
import { CountedRoundsEditor } from './CountedRoundsEditor'
import { PairingSection } from './pairings/PairingSection'
import { TournamentStartPanel } from './TournamentStartPanel'

interface ReadState<T> {
  data: T | undefined
  pending: boolean
  error: Error | null
  retry: () => void
}

interface Props {
  tournament: Tournament
  roster: ReadState<TournamentPlayerRoster>
  rounds: ReadState<Round[]>
}

const dateFormatter = new Intl.DateTimeFormat('nb-NO', { day: 'numeric', month: 'long', year: 'numeric' })
const handicapFormatter = new Intl.NumberFormat('nb-NO', { minimumFractionDigits: 1, maximumFractionDigits: 1 })

function formatDate(value: string): string {
  return dateFormatter.format(new Date(`${value}T12:00:00`))
}

function scoringMode(mode: Tournament['scoring_mode']): string {
  if (mode === 'individual') return 'Individuell'
  if (mode === 'team') return 'Lag'
  return 'Individuell og lag'
}

function formatName(round: Round): string {
  return `Runde ${round.round_number}: ${round.name}`
}

function RoundState({ state, children }: { state: ReadState<Round[]>; children: (rounds: Round[]) => React.ReactNode }) {
  if (state.pending) return <LoadingState />
  if (state.error) return <ErrorState error={state.error} onRetry={state.retry} />
  if (!state.data?.length) return <EmptyState>Ingen runder er opprettet.</EmptyState>
  return children(state.data)
}

function RosterState({ state }: { state: ReadState<TournamentPlayerRoster> }) {
  if (state.pending) return <LoadingState />
  if (state.error) return <ErrorState error={state.error} onRetry={state.retry} />
  if (!state.data?.players.length) return <EmptyState>Ingen deltakere er registrert.</EmptyState>
  return (
    <ul className="management-people">
      {state.data.players.map((player) => (
        <li key={player.player_id}>
          <span>{player.display_name}</span>
          <span>{player.status === 'withdrawn' ? 'Trukket' : `HCP ${handicapFormatter.format(player.tournament_handicap)}`}</span>
        </li>
      ))}
    </ul>
  )
}

export function TournamentManagementSections({ tournament, roster, rounds }: Props) {
  return (
    <div className="management-sections">
      <section id="settings" className="management-section" aria-labelledby="settings-heading" tabIndex={-1}>
        <header><p className="eyebrow">Turneringsfakta</p><h2 id="settings-heading">Innstillinger</h2></header>
        <dl className="management-facts">
          <div><dt>Status</dt><dd><StatusBadge status={tournament.status} /></dd></div>
          <div><dt>Datoer</dt><dd>{formatDate(tournament.start_date)}–{formatDate(tournament.end_date)}</dd></div>
          <div><dt>Poengvisning</dt><dd>{scoringMode(tournament.scoring_mode)}</dd></div>
          <div><dt>Planlagte runder</dt><dd>{tournament.number_of_rounds}</dd></div>
        </dl>
        <CountedRoundsEditor
          tournament={tournament}
          rounds={rounds.data}
          roundsPending={rounds.pending}
          roundsError={rounds.error}
          onRetryRounds={rounds.retry}
        />
        {tournament.description && <p className="management-description">{tournament.description}</p>}
      </section>

      <section id="entrants" className="management-section" aria-labelledby="entrants-heading" tabIndex={-1}>
        <header><p className="eyebrow">Påmeldt spillerliste</p><h2 id="entrants-heading">Deltakere</h2></header>
        <RosterState state={roster} />
      </section>

      <section id="invitations" className="management-section" aria-labelledby="invitations-heading" tabIndex={-1}>
        <header><p className="eyebrow">Tilgang til turneringen</p><h2 id="invitations-heading">Invitasjoner</h2></header>
        <p>Opprett, roter og tilbakekall invitasjonslenker på den eksisterende invitasjonssiden.</p>
        <Link className="management-link" to={`/tournaments/${tournament.id}/invitations`}>
          Åpne invitasjoner <ArrowRight aria-hidden="true" />
        </Link>
      </section>

      <section id="rounds" className="management-section" aria-labelledby="rounds-heading" tabIndex={-1}>
        <header><p className="eyebrow">Turneringsprogram</p><h2 id="rounds-heading">Runder</h2></header>
        <RoundState state={rounds}>{(items) => (
          <ul className="management-links-list">
            {items.map((round) => <li key={round.id}><Link to={`/rounds/${round.id}`}>{formatName(round)}<StatusBadge status={round.status} /></Link></li>)}
          </ul>
        )}</RoundState>
      </section>

      <section id="courses" className="management-section" aria-labelledby="courses-heading" tabIndex={-1}>
        <header><p className="eyebrow">Lagrede rundefakta</p><h2 id="courses-heading">Baner</h2></header>
        <RoundState state={rounds}>{(items) => (
          <CourseConfigurationSection tournamentId={tournament.id} rounds={items} />
        )}</RoundState>
      </section>

      <section id="pairings" className="management-section" aria-labelledby="pairings-heading" tabIndex={-1}>
        <header><p className="eyebrow">Rundespesifikt oppsett</p><h2 id="pairings-heading">Spillegrupper</h2></header>
        <RoundState state={rounds}>{(items) => (
          <PairingSection tournamentId={tournament.id} rounds={items} />
        )}</RoundState>
      </section>

      <section id="lifecycle" className="management-section" aria-labelledby="lifecycle-heading" tabIndex={-1}>
        <header><p className="eyebrow">Gjeldende status</p><h2 id="lifecycle-heading">Livsløp</h2></header>
        <p className="management-current-status">Turneringen er <StatusBadge status={tournament.status} />.</p>
        <TournamentStartPanel tournament={tournament} roster={roster} rounds={rounds} />
        <RoundState state={rounds}>{(items) => (
          <ul className="management-detail-list">
            {items.map((round) => <li key={round.id}><CalendarDays aria-hidden="true" /><span><strong>{formatName(round)}</strong>{formatDate(round.round_date)} · {round.number_of_holes} hull · <StatusBadge status={round.status} /></span></li>)}
          </ul>
        )}</RoundState>
      </section>
    </div>
  )
}
