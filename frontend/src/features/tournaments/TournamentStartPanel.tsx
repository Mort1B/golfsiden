import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { CheckCircle2, CircleAlert, LoaderCircle, LockKeyhole, Play, RefreshCw } from 'lucide-react'
import { tournamentApi, tournamentKeys } from '../../api/tournaments'
import type { Round, Tournament, TournamentPlayerRoster } from '../../api/types'
import { useAuth } from '../auth/authContext'
import {
  tournamentStartFailure,
  tournamentStartReadiness,
  type ReadinessState,
} from './tournamentStart'

interface ReadState<T> {
  data: T | undefined
  pending: boolean
  error: Error | null
  retry: () => void
}

interface TournamentStartPanelProps {
  tournament: Tournament
  rounds: ReadState<Round[]>
  roster: ReadState<TournamentPlayerRoster>
}

const readinessLabels: Record<ReadinessState, string> = {
  pending: 'Kontrollerer …',
  error: 'Kunne ikke kontrolleres',
  ready: 'Klar',
  missing: 'Ikke klar',
}

function ReadinessItem(props: { state: ReadinessState; children: React.ReactNode }) {
  const Icon = props.state === 'ready'
    ? CheckCircle2
    : props.state === 'pending' ? LoaderCircle : CircleAlert
  return (
    <li className={`tournament-start-check ${props.state}`}>
      <Icon aria-hidden="true" />
      <span>{props.children}<small>{readinessLabels[props.state]}</small></span>
    </li>
  )
}

export function TournamentStartPanel(props: TournamentStartPanelProps) {
  const auth = useAuth()
  const queryClient = useQueryClient()
  const userId = auth.session?.user_id ?? ''
  const [receipt, setReceipt] = useState<string | null>(null)
  const readiness = tournamentStartReadiness({
    tournament: props.tournament,
    rounds: props.rounds.data,
    roundsPending: props.rounds.pending,
    roundsError: props.rounds.error,
    roster: props.roster.data,
    rosterPending: props.roster.pending,
    rosterError: props.roster.error,
  })

  const mutation = useMutation({
    mutationFn: () => {
      const csrfToken = auth.session?.csrf_token
      if (!csrfToken) throw new Error('Økten mangler. Logg inn på nytt.')
      return tournamentApi.start(props.tournament.id, {
        expected_tournament_updated_at: props.tournament.updated_at,
      }, csrfToken)
    },
  })

  const refreshAfterFailure = async (refresh: 'none' | 'tournament' | 'all') => {
    if (refresh === 'none') return
    const requests = [queryClient.invalidateQueries({
      queryKey: tournamentKeys.detail(userId, props.tournament.id),
    })]
    if (refresh === 'all') {
      requests.push(
        queryClient.invalidateQueries({ queryKey: tournamentKeys.rounds(userId, props.tournament.id) }),
        queryClient.invalidateQueries({ queryKey: tournamentKeys.players(userId, props.tournament.id) }),
      )
    }
    await Promise.all(requests)
  }

  const start = async () => {
    if (!readiness.canStart || mutation.isPending || props.tournament.status !== 'draft') return
    mutation.reset()
    setReceipt(null)
    try {
      const saved = await mutation.mutateAsync()
      queryClient.setQueryData(tournamentKeys.detail(userId, saved.id), saved)
      setReceipt('Turneringen er startet. Alle rundene er fortsatt i kladd.')
      await queryClient.invalidateQueries({ queryKey: tournamentKeys.root(userId) })
    } catch (error) {
      const failure = tournamentStartFailure(error instanceof Error ? error : new Error('Ukjent feil'))
      await refreshAfterFailure(failure?.refresh ?? 'none')
    }
  }

  const failure = tournamentStartFailure(mutation.error)
  const readFailed = readiness.roundPlan === 'error'
    || readiness.draftRounds === 'error'
    || readiness.activeEntrant === 'error'

  if (props.tournament.status === 'active') {
    return (
      <div className="tournament-start-panel locked">
        <CheckCircle2 aria-hidden="true" />
        <div>
          <h3>Turneringen er startet</h3>
          <p>Rundene er fortsatt i kladd og åpnes separat når hver runde er klar.</p>
          {receipt && <p className="tournament-start-receipt" role="status" aria-live="polite">{receipt}</p>}
        </div>
      </div>
    )
  }

  if (props.tournament.status !== 'draft') {
    return (
      <div className="tournament-start-panel locked">
        <LockKeyhole aria-hidden="true" />
        <div><h3>Start er låst</h3><p>Turneringen kan ikke startes fra denne statusen.</p></div>
      </div>
    )
  }

  return (
    <div className="tournament-start-panel" aria-busy={mutation.isPending}>
      <div><h3>Start turneringen</h3><p>Dette starter selve turneringen. Bane, utslagssted og spillegrupper kontrolleres først når hver runde åpnes. Serveren gjør den endelige startkontrollen.</p></div>
      <ul className="tournament-start-checks" aria-label="Krav før start">
        <ReadinessItem state={readiness.roundPlan}>
          Rundeplan: {readiness.numberedRoundCount} av {props.tournament.number_of_rounds} nummererte runder.
        </ReadinessItem>
        <ReadinessItem state={readiness.draftRounds}>Rundestatus: Alle runder må være i kladd.</ReadinessItem>
        <ReadinessItem state={readiness.activeEntrant}>
          Påmelding: Minst én deltaker må være registrert, ikke trukket og ha en aktiv spillerkonto.
        </ReadinessItem>
      </ul>
      {readFailed && (
        <div className="tournament-start-message error" role="alert">
          <p>Kunne ikke kontrollere alle startkravene. Start er deaktivert til oppdateringen lykkes.</p>
          <button type="button" onClick={() => { props.rounds.retry(); props.roster.retry() }}>
            <RefreshCw aria-hidden="true" /> Prøv kontrollen igjen
          </button>
        </div>
      )}
      {failure && <p className="tournament-start-message error" role="alert">{failure.message}</p>}
      <button
        className="tournament-start-action"
        type="button"
        disabled={!readiness.canStart || mutation.isPending}
        onClick={() => void start()}
      >
        {mutation.isPending ? <LoaderCircle aria-hidden="true" /> : <Play aria-hidden="true" />}
        {mutation.isPending ? 'Starter …' : failure ? 'Prøv å starte igjen' : 'Start turneringen'}
      </button>
      <p className="tournament-start-receipt" aria-live="polite">
        {receipt && <><CheckCircle2 aria-hidden="true" /> {receipt}</>}
      </p>
    </div>
  )
}
