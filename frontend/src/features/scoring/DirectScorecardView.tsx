import { Link } from 'react-router-dom'
import type { ReadScorecard, ScorecardHole } from '../../api/scorecards'
import type { LeaderboardMetric, Round, RoundLeaderboardEntry } from '../../api/types'
import { metricLabel } from '../leaderboards/format'
import { scorecardSearch } from '../leaderboards/drilldownRoutes'
import { ScorecardSummaryView } from './ScorecardSummaryView'
import type { ScoreView } from './selection'

interface Props {
  round: Round
  projectedOwner: RoundLeaderboardEntry
  card: ReadScorecard
  metric: LeaderboardMetric
  view: ScoreView
  hole: ScorecardHole
  onHole: (hole: number) => void
}

export function DirectScorecardView(props: Props) {
  const hidden = props.card.visibility.mode === 'front_nine'
  return (
    <>
      <div className="drilldown-controls">
        <nav aria-label="Resultatberegning">
          <Link aria-current={props.metric === 'gross' ? 'page' : undefined} to={{ search: scorecardSearch('gross', props.view, props.hole.hole_number).toString() }}>Brutto</Link>
          <Link aria-current={props.metric === 'net' ? 'page' : undefined} to={{ search: scorecardSearch('net', props.view, props.hole.hole_number).toString() }}>Netto</Link>
        </nav>
        <nav aria-label="Scorekortvisning">
          <Link aria-current={props.view === 'summary' ? 'page' : undefined} to={{ search: scorecardSearch(props.metric, 'summary').toString() }}>Oppsummering</Link>
          <Link aria-current={props.view === 'hole' ? 'page' : undefined} to={{ search: scorecardSearch(props.metric, 'hole', props.hole.hole_number).toString() }}>Hull</Link>
        </nav>
      </div>
      <header className="scorecard-owner">
        <div><p>{props.projectedOwner.owner.type === 'team' ? 'Lagscore' : 'Individuell score'}</p><h2>{props.projectedOwner.owner_name}</h2></div>
        <span>{props.round.name}</span>
      </header>
      {props.projectedOwner.owner.type === 'team' && props.projectedOwner.members.length > 0 && (
        <p className="direct-card-members">{props.projectedOwner.members.map((member) => member.display_name).join(' · ')}</p>
      )}
      <div className="scoring-notice">Dette er et skrivebeskyttet scorekort.</div>
      {hidden && <div className="scoring-notice warning" role="status">Hull 10–18 er skjult til administratoren frigir finalens bakni.</div>}
      <dl className="scorecard-strip">
        <div><dt>Brutto</dt><dd>{props.card.holes_scored > 0 ? props.card.gross_total : '–'}</dd></div>
        <div><dt>Netto</dt><dd>{props.card.holes_scored > 0 ? props.card.net_total : '–'}</dd></div>
        <div><dt>Synlige hull</dt><dd>{props.card.holes_scored}/{props.card.visible_hole_count}</dd></div>
        <div><dt>Visning</dt><dd>{metricLabel(props.metric)}</dd></div>
      </dl>
      {props.view === 'summary' ? (
        <ScorecardSummaryView card={props.card} disabled={false} readOnly confirming={false}
          confirmationError={null} confirmationRetryable={false} onHole={props.onHole} onConfirm={() => undefined} />
      ) : <DirectHole card={props.card} hole={props.hole} onHole={props.onHole} />}
    </>
  )
}

function DirectHole({ card, hole, onHole }: { card: ReadScorecard; hole: ScorecardHole; onHole: (number: number) => void }) {
  const index = card.holes.findIndex((candidate) => candidate.hole_id === hole.hole_id)
  const previous = card.holes[index - 1]
  const next = card.holes[index + 1]
  return (
    <section className="hole-entry read-hole" aria-labelledby="direct-hole-heading">
      <header><div><p>Hull</p><h2 id="direct-hole-heading">{hole.hole_number}</h2></div><dl><div><dt>Par</dt><dd>{hole.par}</dd></div><div><dt>Index</dt><dd>{hole.stroke_index}</dd></div></dl></header>
      <dl className="score-totals"><div><dt>Brutto</dt><dd>{hole.score?.gross_strokes ?? '–'}</dd></div><div><dt>Netto</dt><dd>{hole.net_strokes ?? '–'}</dd></div></dl>
      <footer>
        <button type="button" disabled={previous === undefined} onClick={() => previous && onHole(previous.hole_number)}>Forrige</button>
        <span>{card.holes_scored} av {card.visible_hole_count} synlige hull registrert</span>
        <button type="button" disabled={next === undefined} onClick={() => next && onHole(next.hole_number)}>Neste</button>
      </footer>
    </section>
  )
}
