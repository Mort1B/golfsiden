import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../../api/client'
import { matchApi, matchKeys, type MatchPair } from '../../api/matchPlay'
import { pairingApi, pairingKeys } from '../../api/pairings'
import { tournamentKeys } from '../../api/tournaments'
import type { Round } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { ErrorState, LoadingState } from '../../ui/AsyncState'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
export function MatchSetup({ round }: { round: Round }) {
  const { session } = useAuth(), user = session?.user_id ?? '', client = useQueryClient()
  const detail = useQuery({ queryKey: tournamentKeys.round(user, round.id), queryFn: () => api.round(round.id) })
  const listing = useQuery({ queryKey: matchKeys.list(user, round.id), queryFn: () => matchApi.list(round.id) })
  const pairings = useQuery({ queryKey: pairingKeys.detail(user, round.id), queryFn: () => pairingApi.get(round.id, round.tournament_id) })
  const [draft, setDraft] = useState<MatchPair[] | null>(null), [message, setMessage] = useState<string | null>(null)
  const pairs = draft ?? listing.data?.matches.map(m => ({ first_player_id: m.opponents[0].player_id, second_player_id: m.opponents[1].player_id })) ?? []
  const current = detail.data ?? round
  const mutation = useMutation({ mutationFn: async (action: 'pairs' | boolean) => {
    if (!session || !detail.data || current.status !== 'draft') throw new Error('Oppsettet er låst eller må oppdateres.')
    if (action === 'pairs') return matchApi.assign(round.id, current.updated_at, pairs, session.csrf_token)
    return matchApi.settings(round.id, current.updated_at, action, session.csrf_token)
  }, onSuccess: async (_result, action) => { if (action === 'pairs') setDraft(null); setMessage('Matchoppsettet er lagret.'); await client.invalidateQueries({ queryKey: privateWorkspaceKeys.user(user) }) }, onError: async () => { await detail.refetch() }, retry: false })
  if (detail.isPending || listing.isPending || pairings.isPending) return <LoadingState />
  const error = detail.error ?? listing.error ?? pairings.error
  if (error) return <ErrorState error={error} onRetry={() => { void detail.refetch(); void listing.refetch(); void pairings.refetch() }} />
  const entrants = pairings.data?.active_entrants ?? []
  const disabled = current.status !== 'draft' || mutation.isPending || detail.isFetching || !session
  const update = (index: number, field: keyof MatchPair, value: string) => { setMessage(null); setDraft(pairs.map((pair, i) => i === index ? { ...pair, [field]: value } : pair)) }
  return <section className="match-panel" aria-label={`Matchoppsett ${round.name}`}>
    <h3>Matchoppsett · {round.name}</h3>
    <p>18 hull fra hull 1. Velg flighter først, og tildel deretter én motstander til hver aktive spiller i samme flight. Ingen motstandere tildeles automatisk.</p>
    <p>Notater kan lagres på enheten etter at matchen er åpnet. Rapportering, gitte hull eller matcher og bekreftelse krever nettforbindelse.</p>
    <label>Offisiell spilleform<select aria-label="Offisiell spilleform" value={current.handicap_enabled ? 'net' : 'gross'} disabled={disabled} onChange={e => { setMessage(null); mutation.mutate(e.target.value === 'net') }}><option value="net">Netto · 100 % relativt spillehandicap</option><option value="gross">Brutto · uten handicapslag</option></select></label>
    <p>Spilleform, motstandere og handicap fryses når runden åpnes.</p>
    {pairs.length === 0 && <p>Ingen matcher er satt opp.</p>}
    {pairs.map((pair, index) => <fieldset key={index}><legend>Match {index + 1}</legend>
      {(['first_player_id', 'second_player_id'] as const).map((field, slot) => <label key={field}>{slot === 0 ? 'Første spiller' : 'Motstander'}<select aria-label={slot === 0 ? 'Første spiller' : 'Motstander'} value={pair[field]} disabled={disabled} onChange={e => update(index, field, e.target.value)}><option value="">Velg spiller</option>{entrants.map(p => <option key={p.player_id} value={p.player_id}>{p.display_name} · {pairings.data?.flights.find(f => f.members.some(m => m.player_id === p.player_id))?.name ?? 'uten flight'}</option>)}</select></label>)}
      <button type="button" disabled={disabled} onClick={() => setDraft(pairs.filter((_, i) => i !== index))}>Fjern match {index + 1}</button>
    </fieldset>)}
    <div className="match-actions"><button type="button" disabled={disabled} onClick={() => setDraft([...pairs, { first_player_id: '', second_player_id: '' }])}>Legg til match</button>
      <button type="button" disabled={disabled || draft === null || pairs.some(p => !p.first_player_id || !p.second_player_id || p.first_player_id === p.second_player_id)} onClick={() => mutation.mutate('pairs')}>{mutation.isPending ? 'Lagrer …' : 'Lagre motstandere'}</button></div>
    {mutation.error && <p role="alert">{mutation.error.message}</p>}{message && <p role="status">{message}</p>}
  </section>
}
