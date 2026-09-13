import { useRef, useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { api } from '../../api/client'
import { tournamentKeys } from '../../api/tournaments'
import { matchApi, matchKeys } from '../../api/matchPlay'
import { roundLifecycleApi, roundLifecycleKeys } from '../../api/roundLifecycle'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import type { Round, Tournament } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { OpeningReadiness } from '../tournaments/lifecycle/RoundReadiness'
import { StatusBadge } from '../../ui/StatusBadge'
import { LoadingState, ErrorState } from '../../ui/AsyncState'
import { matchUrl } from './format'
export function MatchLifecycle({ tournament, round, authorityRefreshing = false }: { tournament: Tournament; round: Round; authorityRefreshing?: boolean }) {
  const { session } = useAuth(), user = session?.user_id ?? '', client = useQueryClient(), [confirm, setConfirm] = useState(false)
  const actionButton = useRef<HTMLButtonElement>(null)
  const cancel = () => { setConfirm(false); actionButton.current?.focus() }
  const detail = useQuery({ queryKey: tournamentKeys.round(user, round.id), queryFn: () => api.round(round.id), staleTime: 0 })
  const current = detail.data ?? round
  const opening = useQuery({ queryKey: roundLifecycleKeys.validation(user, round.id), queryFn: () => roundLifecycleApi.validation(round.id), enabled: current.status === 'draft' })
  const completion = useQuery({ queryKey: matchKeys.completion(user, round.id), queryFn: () => matchApi.completion(round.id), enabled: current.status !== 'draft' })
  const action = current.status === 'draft' ? 'open' : current.status === 'open' ? 'complete' : 'lock'
  const label = action === 'open' ? 'Åpne runden' : action === 'complete' ? 'Fullfør runden' : 'Lås runden'
  const ready = current.status === 'draft' ? opening.data?.ready && tournament.status === 'active' : current.status === 'open' ? completion.data?.ready_to_complete : completion.data?.ready_to_lock
  const mutation = useMutation({ mutationFn: async () => { if (!session) throw new Error('Økten mangler'); return roundLifecycleApi.transition(round.id, round.tournament_id, action, session.csrf_token) }, onSettled: async () => { setConfirm(false); await client.invalidateQueries({ queryKey: privateWorkspaceKeys.user(user) }) }, retry: false })
  const error = detail.error ?? opening.error ?? completion.error
  const pending = detail.isFetching || (current.status === 'draft' ? opening.isFetching : completion.isFetching)
  return <section className="match-panel"><h4>{current.name} · <StatusBadge status={current.status} /></h4>
    {error && <ErrorState error={error} onRetry={() => { void detail.refetch(); if (current.status === 'draft') void opening.refetch(); else void completion.refetch() }} />}
    {pending && <LoadingState />}
    {opening.data && current.status === 'draft' && <OpeningReadiness round={current} validation={opening.data} />}
    {current.status !== 'draft' && <><p>Alle matcher må være avsluttet og bekreftet før fullføring og låsing. Uspilte hull etter en tidlig avslutning krever ingen notater.</p>
      <p>{completion.data?.matches.filter(m => m.confirmed === true).length ?? 0} av {completion.data?.matches.length ?? 0} matcher bekreftet.</p><Link to={matchUrl(round.id)}>Se matcher og bekreftelser</Link></>}
    {current.status === 'locked' ? <p>Runden er låst. Administrator kan registrere begrunnede korrigeringer i matchen.</p> : <>
      <button ref={actionButton} type="button" disabled={!ready || pending || !!error || authorityRefreshing || mutation.isPending} onClick={() => setConfirm(true)}>{label}</button>
      {confirm && <fieldset onKeyDown={e => { if (e.key === 'Escape') cancel() }}><legend>Bekreft {label.toLowerCase()}</legend><p>{current.name}. Åpning fryser oppsettet; låsing stenger ordinære endringer.</p><button type="button" autoFocus onClick={cancel}>Avbryt</button><button type="button" disabled={!ready || pending || !!error || authorityRefreshing || mutation.isPending} onClick={() => mutation.mutate()}>Bekreft og {label.toLowerCase()}</button></fieldset>}
    </>}{mutation.error && <p role="alert">{mutation.error.message}</p>}{mutation.isSuccess && <p role="status">Rundestatus er oppdatert.</p>}
  </section>
}
