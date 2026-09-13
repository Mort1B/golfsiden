import type { PublicResults } from '../../api/resultSharing'
import { metricLabel, positionLabel, scoreToParLabel } from '../leaderboards/format'
import { tieBreakExplanation, tieBreakLabel } from '../leaderboards/tieBreakExplanation'

export function PublicStandings({ board }: { board: PublicResults }) {
  return <section className="public-standings" aria-label="Delte sammenlagtresultater">
    <h1>{board.tournament_name}</h1>
    <h2>{metricLabel(board.metric)} sammenlagt</h2>
    <p>Beste {board.required_counted_rounds} av {board.final_round_number} runder.</p>
    <p><strong>Lik totalscore: {tieBreakLabel(board.tie_break_policy)}.</strong> {tieBreakExplanation(board.tie_break_policy)}</p>
    {board.visibility.mode === 'front_nine' && <p className="public-result-notice">Finalens bakni er skjult. Bare synlige resultater inngår her.</p>}
    {board.entries.length === 0 ? <p>Ingen resultater å vise ennå.</p> : <ol className="public-result-list">
      {board.entries.map((entry, index) => <li key={index} className="public-result-row">
        <span className="public-result-place" aria-label={entry.position === null ? 'Uten plassering' : `${entry.tied ? 'Delt ' : ''}plass ${entry.position}`}>{positionLabel(entry.position, entry.tied)}</span>
        <div className="public-result-player"><h3>{entry.display_name}</h3>
          <p>Kvalifisering: {entry.counted_contributions} av {board.required_counted_rounds} · {entry.eligible ? 'Kvalifisert' : 'Ikke kvalifisert ennå'}</p>
          <p>{entry.completed_rounds} fullførte runder</p>
          {entry.provisional && <p>Foreløpig · {entry.provisional_holes_scored} hull ført i tellende åpen runde</p>}
          {entry.tie_break_score_to_par !== null && <p>Siste runde: {scoreToParLabel(entry.tie_break_score_to_par)} {metricLabel(board.metric).toLowerCase()} · sammenlignet ved lik totalscore</p>}
        </div>
        <div className="public-result-score"><strong>{entry.position === null ? '–' : scoreToParLabel(entry.score_to_par)}</strong><span>{entry.position === null ? 'Ingen tellende score' : `${entry.total} ${metricLabel(board.metric).toLowerCase()}`}</span></div>
      </li>)}
    </ol>}
  </section>
}
