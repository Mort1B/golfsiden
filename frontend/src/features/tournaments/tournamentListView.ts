import type { Tournament } from '../../api/types'

export const TOURNAMENT_LIST_VIEWS = [
  { id: 'current', label: 'Nåværende' },
  { id: 'archived', label: 'Arkiv' },
  { id: 'all', label: 'Alle' },
] as const
export type TournamentListView = typeof TOURNAMENT_LIST_VIEWS[number]['id']

export function tournamentListView(params: URLSearchParams): TournamentListView {
  return TOURNAMENT_LIST_VIEWS.find((view) => view.id === params.get('view'))?.id ?? 'current'
}
export function matchesTournamentView(tournament: Tournament, view: TournamentListView): boolean {
  return view === 'all' || (view === 'archived' ? tournament.status === 'archived' : tournament.status !== 'archived')
}
export function tournamentViewSearch(params: URLSearchParams, view: TournamentListView): string {
  const next = new URLSearchParams(params)
  if (view === 'current') next.delete('view')
  else next.set('view', view)
  return next.size ? `?${next.toString()}` : ''
}
