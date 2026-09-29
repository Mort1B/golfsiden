import { useLayoutEffect, useRef, useState } from 'react'
import { authKeys, type AuthSession } from '../../api/auth'
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
  const { session } = useAuth()
  if (!session) return null
  return <OwnedStartPanel key={`${session.user_id}:${session.csrf_token}:${props.tournament.id}`} {...props} session={session} />
}

function OwnedStartPanel(props: TournamentStartPanelProps & { session: AuthSession }) {
  const queryClient = useQueryClient()
  const { session } = props
  const userId = session.user_id
  const alive = useRef(false), busy = useRef(false)
  useLayoutEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const isCurrent = () => {
    const current = queryClient.getQueryData<AuthSession | null>(authKeys.session)
    return alive.current && current?.user_id === userId && current.csrf_token === session.csrf_token
  }
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
    retry: false, gcTime: 0,
    mutationFn: () => {
      const csrfToken = session.csrf_token
      if (!csrfToken || !isCurrent()) throw new Error('Økten mangler. Logg inn på nytt.')
      return tournamentApi.start(props.tournament.id, {
        expected_tournament_updated_at: props.tournament.updated_at,
      }, csrfToken)
    },
  })

  const refreshAfterFailure = async (refresh: 'none' | 'tournament' | 'all') => {
    if (refresh === 'none') return
    const keys: ReadonlyArray<readonly unknown[]> = [
      tournamentKeys.detail(userId, props.tournament.id),
      ...(refresh === 'all' ? [tournamentKeys.rounds(userId, props.tournament.id),
        tournamentKeys.players(userId, props.tournament.id)] : []),
    ]
    await Promise.all(keys.map(queryKey => isCurrent()
      ? queryClient.invalidateQueries({ queryKey }) : Promise.resolve()))
  }

  const start = async () => {
    if (!isCurrent() || busy.current || !readiness.canStart || mutation.isPending || props.tournament.status !== 'draft') return
    busy.current = true
    mutation.reset()
    setReceipt(null)
    try {
      const saved = await mutation.mutateAsync()
      if (!isCurrent()) return
      queryClient.setQueryData(tournamentKeys.detail(userId, saved.id), saved)
      if (!isCurrent()) return
      setReceipt('Turneringen er startet. Alle rundene er fortsatt i kladd.')
      if (isCurrent()) await queryClient.invalidateQueries({ queryKey: tournamentKeys.root(userId) })
    } catch (error) {
      if (!isCurrent()) return
      const failure = tournamentStartFailure(error instanceof Error ? error : new Error('Ukjent feil'))
      await refreshAfterFailure(failure?.refresh ?? 'none')
    } finally {
      if (isCurrent()) busy.current = false
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
          <p>Rundene åpnes separat når hver runde er klar.</p>
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
          <button type="button" onClick={() => { if (isCurrent()) props.rounds.retry(); if (isCurrent()) props.roster.retry() }}>
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
