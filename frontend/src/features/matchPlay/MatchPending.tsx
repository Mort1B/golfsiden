import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { matchApi, matchKeys, type MatchCard, type MatchScoringCard } from '../../api/matchPlay'
import { ApiHttpError } from '../../api/http'
import { useMatchQueue } from './offline/context'
import { matchDatabase } from './offline/database'
import type { MatchDraft } from './offline/model'
import { matchUrl } from './format'
export function MatchPendingLink() {
  const queue = useMatchQueue(), pending = queue.items.filter(i => i.notes.length || i.action)
  return pending.length ? <aside className="match-pending" aria-label="Lokale matchendringer"><p>{pending.length} matcher har lokale endringer eller levering som må kontrolleres.</p>{pending.map(i => <Link key={i.key} to={matchUrl(i.roundId, i.matchId, true)}>Kontroller matchnotater</Link>)}</aside> : null
}
export function MatchPending({ item }: { item: MatchDraft }) {
  const queue = useMatchQueue(), [review, setReview] = useState(false), [error, setError] = useState<string | null>(null)
  const canonical = useQuery<MatchCard | MatchScoringCard>({ queryKey: [...matchKeys.scoring(item.accountId, item.roundId, item.matchId), 'conflict-review'],
    queryFn: async () => { try { return await matchApi.scoring(item.roundId, item.matchId) } catch (e) { if (e instanceof ApiHttpError && e.status === 403) return matchApi.read(item.roundId, item.matchId); throw e } }, enabled: review && queue.online, staleTime: 0, retry: false })
  const permitted = !canonical.error && !canonical.isFetching ? canonical.data : undefined
  const uncertain = item.action?.phase === 'unknown' || item.action?.phase === 'acknowledged' || item.notes.some(n => n.phase === 'unknown' || n.phase === 'acknowledged')
  const run = async (keep: boolean) => {
    try {
      if (!canonical.data || canonical.isFetching || canonical.error) throw new Error('Kontroller gjeldende match først.')
      const card = canonical.data
      if (keep && (!('revision' in card) || card.finish || card.round_status === 'locked')) throw new Error('Matchen er avsluttet eller låst; notatene kan ikke sendes.')
      await matchDatabase.resolve(item.key, item.generation, 'revision' in card ? card.revision : '1', keep)
      await queue.runtime.changed(); setReview(false); setError(null)
    } catch (e) { setError(e instanceof Error ? e.message : 'Kunne ikke behandle notatene.') }
  }
  return <section className="match-panel" aria-label="Lokale matchendringer"><h3>Lokale endringer og leveringsstatus</h3>
    {item.action && <p role="status">{item.action.phase === 'acknowledged' ? 'Handlingen er mottatt. Gjeldende match må kontrolleres.' : item.action.phase === 'blocked' ? 'Handlingen ble avvist. Kontroller før du prøver igjen.' : 'Levering av handling er ukjent; original forespørsel beholdes.'}</p>}
    {item.notes.map(n => <p key={n.request.request_id}>{n.request.command.type === 'note' ? `${n.request.command.gross_strokes} slag på hull ${n.request.command.hole_number}` : n.request.command.type === 'clear_note' ? `Tøm notat på hull ${n.request.command.hole_number}` : 'Handling'} · {n.phase === 'queued' ? 'Lagret på enheten' : n.phase === 'acknowledged' ? 'Mottatt; kontroll venter' : n.phase === 'unknown' ? 'Ukjent levering' : 'Stoppet for gjennomgang'}</p>)}
    <button type="button" onClick={() => { void matchDatabase.retry(item.key).then(queue.runtime.changed).catch(e => setError(e instanceof Error ? e.message : 'Kunne ikke prøve igjen')) }}>Kontroller levering igjen</button>
    <button type="button" aria-expanded={review} onClick={() => setReview(!review)}>Se gammel, lokal og gjeldende verdi</button>
    {review && <div>{canonical.isFetching && <p role="status">Kontrollerer server …</p>}{canonical.error && <p role="alert">{canonical.error.message}</p>}
      <button type="button" onClick={() => void canonical.refetch()}>Oppdater gjeldende match</button>
      {item.notes.map(n => { const c = n.request.command; if (c.type !== 'note' && c.type !== 'clear_note') return null
        const hidden = permitted?.visibility.mode === 'front_nine' && c.hole_number > 9
        const server = permitted?.notes.find(v => v.player_id === c.player_id && v.hole_number === c.hole_number)?.gross_strokes
        return <p key={n.request.request_id}>{permitted?.opponents.find(p => p.player_id === c.player_id)?.display_name ?? 'Spiller'} · hull {c.hole_number}: gammel {n.oldValue ?? 'tomt'}, lokal {c.type === 'note' ? c.gross_strokes : 'tomt'}, gjeldende {!permitted ? 'ikke kontrollert' : hidden ? 'skjult' : server ?? 'tomt'}.</p>
      })}
      {uncertain ? <p>Den opprinnelige leveransen må avklares før du velger ny verdi eller forkaster.</p> : <div className="match-actions"><button type="button" disabled={!canonical.data || canonical.isFetching || !!canonical.error} onClick={() => void run(false)}>Behold serverens verdier og forkast lokale endringer</button>
        <button type="button" disabled={!canonical.data || !('revision' in canonical.data) || canonical.isFetching || !!canonical.error || !!canonical.data.finish || canonical.data.round_status === 'locked' || item.notes.length === 0} onClick={() => void run(true)}>Bruk gjennomgåtte lokale notater</button></div>}
    </div>}{error && <p role="alert">{error}</p>}
  </section>
}
