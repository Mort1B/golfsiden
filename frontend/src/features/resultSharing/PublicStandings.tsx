import { overallSelected, overallTieBreak } from '../../api/leaderboards/values'
import type { PublicResults } from '../../api/resultSharing'
import { metricLabel, positionLabel, scoreToParLabel } from '../leaderboards/format'
import { tieBreakExplanation, tieBreakLabel } from '../leaderboards/tieBreakExplanation'

export function PublicStandings({ board }: { board: PublicResults }) {
  return <section className="public-standings" aria-label="Delte sammenlagtresultater">
    <h1>{board.tournament_name}</h1>
    <h2>{metricLabel(board.metric)} sammenlagt</h2>
    <p>Beste {board.required_counted_rounds} av {board.final_round_number} runder.</p>
    {board.entries.some(entry => entry.value) && <p>Sammenlagtekvivalent: Stableford omregnes til 36 minus poeng på fullt kort, ellers 2 per løste hull minus poeng. Slagspill bidrar med score mot par. Lavest resultat vinner.</p>}
    <p><strong>Lik totalscore: {tieBreakLabel(board.tie_break_policy)}.</strong> {tieBreakExplanation(board.tie_break_policy)}</p>
    {board.visibility.mode === 'front_nine' && <p className="public-result-notice">Finalens bakni er skjult. Bare synlige resultater inngår her.</p>}
    {board.entries.length === 0 ? <p>Ingen resultater å vise ennå.</p> : <ol className="public-result-list">
      {board.entries.map((entry, index) => <li key={index} className="public-result-row">
        <span className="public-result-place" aria-label={entry.position === null ? 'Uten plassering' : `${entry.tied ? 'Delt ' : ''}plass ${entry.position}`}>{positionLabel(entry.position, entry.tied)}</span>
        <div className="public-result-player"><h3>{entry.display_name}</h3>
          <p>Kvalifisering: {entry.counted_contributions} av {board.required_counted_rounds} · {entry.eligible ? 'Kvalifisert' : 'Ikke kvalifisert ennå'}</p>
          <p>{entry.completed_rounds} fullførte runder</p>
          {entry.provisional && <p>Foreløpig · {entry.provisional_holes_scored} hull ført i tellende åpen runde</p>}
          {overallTieBreak(entry) !== null && <p>Siste runde{entry.value ? ' (ekvivalent)' : ''}: {scoreToParLabel(overallTieBreak(entry) ?? 0)} {metricLabel(board.metric).toLowerCase()} · sammenlignet ved lik totalscore</p>}
        </div>
        <div className="public-result-score"><strong>{entry.position === null ? '–' : scoreToParLabel(overallSelected(entry))}</strong><span>{entry.position === null ? 'Ingen tellende score' : `${entry.value ? 'Omregnet' : entry.total} ${metricLabel(board.metric).toLowerCase()}`}</span></div>
      </li>)}
    </ol>}
  </section>
}
