import { StrictMode, type ReactNode } from 'react'
import { act, render, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { vi } from 'vitest'
import { authKeys, type AuthSession } from '../../../api/auth'
import { AuthContext, type AuthContextValue } from '../../auth/authContext'
import { publishSessionTransition } from '../../auth/sessionTransition'
import { round, session, tournament as initialTournament } from '../lifecycle/__tests__/fixtures'
import type { TournamentPlayerRoster } from '../../../api/types'
import type { RoundPairings } from '../../../api/pairings'
export { round, session }
export const tournament = { ...initialTournament, status: 'draft' as const }
export const roster: TournamentPlayerRoster = { handicap_correction: { state: 'editable' }, players: [{
  tournament_id: tournament.id, player_id: '00000000-0000-0000-0000-000000000004', display_name: 'Deltaker',
  player_active: true, tournament_handicap: 10, seed: null, status: 'active',
  created_at: tournament.created_at, updated_at: tournament.updated_at,
}] }
export const pairings: RoundPairings = { round_id: round.id, tournament_id: tournament.id,
  status: 'draft', scoring_format: 'individual_stroke_play', updated_at: round.updated_at,
  active_entrants: [], inactive_entrants: [], teams: [], legacy_individual_groups: [],
  flights: [{ id: '00000000-0000-0000-0000-000000000010', name: 'Opprinnelig flight', starting_hole: null,
    tee_time: null, created_at: round.created_at, updated_at: round.updated_at, members: [] }],
}
export const savedPairings = { ...pairings, updated_at: '2026-09-08T12:00:00Z',
  flights: pairings.flights.map(flight => ({ ...flight, name: 'Lagret flight' })) }
export function deferred<T>() {
  let resolve: (value: T) => void = () => undefined, reject: (error: unknown) => void = () => undefined
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
export function mountEditor(client: QueryClient, child: (target: string) => ReactNode) {
  client.setQueryData(authKeys.session, session)
  const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
  const tree = (identity: AuthSession | null, target: string, mounted: boolean) => <StrictMode><QueryClientProvider client={client}>
    <AuthContext value={{ ...auth, session: identity }}>{mounted && child(target)}</AuthContext>
  </QueryClientProvider></StrictMode>
  const view = render(tree(session, 'initial', true))
  return { change: (identity: AuthSession | null, target = 'initial', mounted = true) => {
    publishSessionTransition(client, identity); view.rerender(tree(identity, target, mounted))
  } }
}
export async function transition(change: ReturnType<typeof mountEditor>['change'], kind: string) {
  const next = kind === 'logout' ? null : kind === 'account' ? { ...session, user_id: crypto.randomUUID(), csrf_token: 'other' }
    : kind === 'csrf' ? { ...session, csrf_token: 'renewed' } : session
  await act(async () => change(next, ['target', 'round', 'tournament'].includes(kind) ? kind : 'initial', !['logout', 'unmount'].includes(kind)))
}
export async function settled(client: QueryClient) {
  await waitFor(() => { if (client.getMutationCache().getAll().some(m => m.state.status === 'pending')) throw new Error('mutation pending') })
}
