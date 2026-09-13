import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { matchApi, matchKeys, type MatchCard } from '../../api/matchPlay'
import { useAuth } from '../auth/authContext'
import { EmptyState, ErrorState, LoadingState } from '../../ui/AsyncState'
import { eventLabel, matchResult, matchUrl, pointsLabel, tableUrl } from './format'
export function MatchRound({ roundId, playerId }: { roundId: string; playerId?: string }) {
  const user = useAuth().session?.user_id ?? ''
  const query = useQuery({ queryKey: matchKeys.list(user, roundId), queryFn: () => matchApi.list(roundId), retry: false })
  if (query.error) return <ErrorState error={query.error} onRetry={() => void query.refetch()} />
  if (!query.data) return <LoadingState />
  const cards = query.data.matches.filter(m => !playerId || m.opponents.some(p => p.player_id === playerId))
  return <section className="match-list" aria-label="Matcher">
    {cards.length === 0 && <EmptyState>Ingen matcher er satt opp for dette valget.</EmptyState>}
    {query.isFetching && <p role="status">Oppdaterer matcher …</p>}
    {cards.map(card => <article key={card.match_id} className="match-panel">
      <h3>{card.opponents[0].display_name} mot {card.opponents[1].display_name}</h3>
      <p>{card.mode === 'net' ? 'Netto' : 'Brutto'} · {matchResult(card)}</p>
      <p>{card.confirmed === null ? 'Fullføring og poeng er skjult til finalen frigis.' : card.confirmed ? 'Bekreftet resultat' : card.finish ? 'Avsluttet · venter på bekreftelse' : 'Ingen matchpoeng tildelt'}</p>
      <div className="match-actions"><Link to={matchUrl(roundId, card.match_id)}>Les match</Link>
        {query.data.writable_match_ids.includes(card.match_id) && <Link to={matchUrl(roundId, card.match_id, true)}>Før match</Link>}
        <Link to={tableUrl(card.tournament_id)}>Matchpoeng</Link></div>
    </article>)}
  </section>
}
export function MatchReadView({ card }: { card: MatchCard }) {
  return <section className="match-panel">
    <h2>{card.opponents[0].display_name} mot {card.opponents[1].display_name}</h2>
    <p>Offisiell spilleform: {card.mode === 'net' ? 'Netto' : 'Brutto'} · Spille-HCP {card.opponents.map(p => p.playing_handicap ?? 'ikke fryst').join(' / ')} · Relative slag {card.relative_handicaps.join(' / ')}</p>
    <h3>{matchResult(card)}</h3>
    {card.visibility.mode === 'front_nine' && <p role="status">Bare hull 1–9 vises. Fullføring, bekreftelse og matchpoeng er skjult til finalen frigis.</p>}
    {card.correction_pending && <p role="status">Korrigert resultat venter på ny bekreftelse. Ingen poeng tildelt.</p>}
    {card.half_points ? <p>Bekreftet · matchpoeng {card.half_points.map(pointsLabel).join(' / ')}</p> : card.confirmed !== null && <p>Ingen bekreftede matchpoeng ennå.</p>}
    <h3>Aksepterte rapporter</h3>{card.events.length ? <ol>{card.events.map((e, i) => <li key={i}>{eventLabel(e, card)}</li>)}</ol> : <p>Ingen hull eller matchresultat er rapportert.</p>}
    <details><summary>Numeriske notater og hull</summary><p>Notater er forslag. De endrer ikke en akseptert rapport.</p><ol className="match-holes">{card.holes.map(h => <li key={h.hole_number}><strong>Hull {h.hole_number} · par {h.par} · indeks {h.stroke_index}</strong><span>{card.finish && h.hole_number > card.resolved_holes ? 'Uspilt etter matchslutt' : card.opponents.map(p => `${p.display_name}: ${card.notes.find(n => n.hole_number === h.hole_number && n.player_id === p.player_id)?.gross_strokes ?? 'tomt'}`).join(' / ')}</span></li>)}</ol></details>
  </section>
}
