import type { OwnerCompletionProgress, ReadScorecard, ScorecardHole } from '../../api/scorecards'
import type { Round, Tournament } from '../../api/types'
import { ScoreSelectors } from './ScoreSelectors'
import { ScorecardSummaryView } from './ScorecardSummaryView'
import type { ScoreView } from './selection'

interface ReadScorecardExperienceProps {
  tournaments: Tournament[]
  rounds: Round[]
  round: Round
  owners: OwnerCompletionProgress[]
  selectedOwner: OwnerCompletionProgress
  card: ReadScorecard
  hole: ScorecardHole
  view: ScoreView
  onTournament: (id: string) => void
  onRound: (id: string) => void
  onOwner: (id: string) => void
  onHole: (number: number) => void
  onView: (view: ScoreView) => void
}

export function ReadScorecardExperience(props: ReadScorecardExperienceProps) {
  const hidden = props.card.visibility.mode === 'front_nine'
  return (
    <>
      <ScoreSelectors
        tournaments={props.tournaments}
        rounds={props.rounds}
        owners={props.owners}
        holes={props.card.holes}
        tournamentId={props.round.tournament_id}
        roundId={props.round.id}
        owner={props.selectedOwner.owner}
        holeNumber={props.hole.hole_number}
        view={props.view}
        disabled={false}
        onTournament={props.onTournament}
        onRound={props.onRound}
        onOwner={props.onOwner}
        onHole={props.onHole}
        onView={props.onView}
      />
      <header className="scorecard-owner">
        <div><p>{props.selectedOwner.owner.type === 'team' ? 'Lagscore' : 'Individuell score'}</p><h2>{props.selectedOwner.owner_name}</h2></div>
        <span>{props.round.name}</span>
      </header>
      <div className="scoring-notice">Du kan se dette scorekortet, men ikke føre score for det.</div>
      {hidden && <div className="scoring-notice warning" role="status">Hull 10–18 er skjult til administratoren frigir finalens bakni.</div>}
      <dl className="scorecard-strip">
        <div><dt>Brutto</dt><dd>{props.card.holes_scored > 0 ? props.card.gross_total : '–'}</dd></div>
        <div><dt>Netto</dt><dd>{props.card.holes_scored > 0 ? props.card.net_total : '–'}</dd></div>
        <div><dt>Synlige hull</dt><dd>{props.card.holes_scored}/{props.card.visible_hole_count}</dd></div>
        <div><dt>Spille-HCP</dt><dd>{props.card.playing_handicap}</dd></div>
      </dl>
      {props.view === 'hole' ? <ReadHole card={props.card} hole={props.hole} onHole={props.onHole} /> : (
        <ScorecardSummaryView card={props.card} disabled={false} readOnly confirming={false}
          confirmationError={null} confirmationRetryable={false} onHole={props.onHole} onConfirm={() => undefined} />
      )}
    </>
  )
}

function ReadHole({ card, hole, onHole }: { card: ReadScorecard; hole: ScorecardHole; onHole: (number: number) => void }) {
  return (
    <section className="hole-entry read-hole" aria-labelledby="current-hole-heading">
      <header><div><p>Hull</p><h2 id="current-hole-heading">{hole.hole_number}</h2></div><dl><div><dt>Par</dt><dd>{hole.par}</dd></div><div><dt>Index</dt><dd>{hole.stroke_index}</dd></div></dl></header>
      <dl className="score-totals"><div><dt>Brutto</dt><dd>{hole.score?.gross_strokes ?? '–'}</dd></div><div><dt>Netto</dt><dd>{hole.net_strokes ?? '–'}</dd></div></dl>
      <footer>
        <button type="button" disabled={hole.hole_number === 1} onClick={() => onHole(hole.hole_number - 1)}>Forrige</button>
        <span>{card.holes_scored} av {card.visible_hole_count} synlige hull registrert</span>
        <button type="button" disabled={hole.hole_number === card.holes.length} onClick={() => onHole(hole.hole_number + 1)}>Neste</button>
      </footer>
    </section>
  )
}
