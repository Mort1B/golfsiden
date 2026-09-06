import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { api } from '../../../api/client'
import { privateWorkspaceKeys } from '../../../api/privateWorkspace'
import { roundLifecycleApi, roundLifecycleKeys } from '../../../api/roundLifecycle'
import type { Round, Tournament } from '../../../api/types'
import { useAuth } from '../../auth/authContext'
import { completionAttention, openingAttention } from './roundAttention'

interface Props { tournament: Tournament; round: Round; blocked: boolean; onRefresh: () => void }

export function OrganizerRound({ tournament, round, blocked, onRefresh }: Props) {
  const userId = useAuth().session?.user_id ?? ''
  const opening = useQuery({
    queryKey: roundLifecycleKeys.validation(userId, round.id),
    queryFn: () => roundLifecycleApi.validation(round.id),
    enabled: round.status === 'draft', staleTime: 0, refetchOnMount: 'always', retry: false,
  })
  const completion = useQuery({
    queryKey: privateWorkspaceKeys.completion(userId, round.id),
    queryFn: () => api.completionValidation(round.id, round.scoring_format),
    enabled: round.status === 'open' || round.status === 'completed', staleTime: 0, refetchOnMount: 'always', retry: false,
  })
  const query = round.status === 'draft' ? opening : completion
  if (blocked) return null
  const unavailable = query.error || query.fetchStatus === 'paused'
  const inconsistent = round.status === 'draft' ? opening.data?.round_id !== round.id
    : completion.data?.round_id !== round.id || completion.data.status !== round.status || completion.data.visibility.mode !== 'full'
  const title = `Runde ${round.round_number}: ${round.name}`
  if (unavailable) return <li><p role="alert">Oppfølging for {title} kunne ikke hentes.</p><button type="button" onClick={onRefresh}>Prøv igjen for runde {round.round_number}</button></li>
  if (query.isPending || query.isFetching) return <li><p role="status">Kontrollerer {title} …</p></li>
  if (inconsistent) return <li><p role="alert">Status for {title} må oppdateres før oppfølging kan vises.</p><button type="button" onClick={onRefresh}>Oppdater runde {round.round_number}</button></li>
  const items = round.status === 'draft' && opening.data ? openingAttention(tournament, round, opening.data)
    : completion.data ? completionAttention(round, completion.data) : []
  return <li><h3>{title}</h3><ul>{items.map(item => <li key={item.text}>
    <p>{item.text}</p><Link to={item.href}>{item.link}</Link>
  </li>)}</ul></li>
}
