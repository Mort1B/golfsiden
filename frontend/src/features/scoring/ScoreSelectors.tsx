import type { OwnerCompletionProgress, ScoreOwner, ScorecardHole } from '../../api/scorecards'
import type { Round, Tournament } from '../../api/types'
import type { ScoreView } from './selection'

interface ScoreSelectorsProps {
  tournaments: Tournament[]
  rounds: Round[]
  owners: OwnerCompletionProgress[]
  holes: ScorecardHole[]
  tournamentId: string
  roundId: string
  owner: ScoreOwner
  holeNumber: number
  view: ScoreView
  disabled: boolean
  onTournament: (id: string) => void
  onRound: (id: string) => void
  onOwner: (id: string) => void
  onHole: (number: number) => void
  onView: (view: ScoreView) => void
}

export function ScoreSelectors(props: ScoreSelectorsProps) {
  return (
    <section className="score-selectors" aria-label="Velg scorekort">
      <label><span>Turnering</span><select disabled={props.disabled} value={props.tournamentId} onChange={(event) => props.onTournament(event.target.value)}>
        {props.tournaments.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
      </select></label>
      <label><span>Runde</span><select disabled={props.disabled} value={props.roundId} onChange={(event) => props.onRound(event.target.value)}>
        {props.rounds.map((item) => <option key={item.id} value={item.id}>Runde {item.round_number}: {item.name}</option>)}
      </select></label>
      <label><span>{props.owner.type === 'team' ? 'Lag' : 'Spiller'}</span><select disabled={props.disabled} value={props.owner.id} onChange={(event) => props.onOwner(event.target.value)}>
        {props.owners.map((item) => <option key={`${item.owner.type}-${item.owner.id}`} value={item.owner.id}>{item.owner_name}</option>)}
      </select></label>
      <label><span>Hull</span><select disabled={props.disabled} value={props.holeNumber} onChange={(event) => props.onHole(Number(event.target.value))}>
        {props.holes.map((hole) => <option key={hole.hole_id} value={hole.hole_number}>Hull {hole.hole_number}</option>)}
      </select></label>
      <fieldset className="score-view-toggle">
        <legend>Visning</legend>
        <div>
          <button type="button" disabled={props.disabled} aria-pressed={props.view === 'hole'} onClick={() => props.onView('hole')}>Ett hull</button>
          <button type="button" disabled={props.disabled} aria-pressed={props.view === 'summary'} onClick={() => props.onView('summary')}>Oppsummering</button>
        </div>
      </fieldset>
    </section>
  )
}
