import type { LeaderboardMetric, Round, Tournament } from '../../api/types'
import type { LeaderboardScope } from './selection'

interface LeaderboardControlsProps {
  tournaments: Tournament[]
  rounds: Round[]
  tournamentId: string
  roundId: string | undefined
  scope: LeaderboardScope
  metric: LeaderboardMetric
  roundsPending: boolean
  onTournamentChange: (id: string) => void
  onRoundChange: (id: string) => void
  onScopeChange: (scope: LeaderboardScope) => void
  onMetricChange: (metric: LeaderboardMetric) => void
}

export function LeaderboardControls({
  tournaments,
  rounds,
  tournamentId,
  roundId,
  scope,
  metric,
  roundsPending,
  onTournamentChange,
  onRoundChange,
  onScopeChange,
  onMetricChange,
}: LeaderboardControlsProps) {
  return (
    <section className="leaderboard-controls" aria-label="Resultatvisning">
      <label className="leaderboard-select">
        <span>Turnering</span>
        <select value={tournamentId} onChange={(event) => onTournamentChange(event.target.value)}>
          {tournaments.map((tournament) => (
            <option key={tournament.id} value={tournament.id}>{tournament.name}</option>
          ))}
        </select>
      </label>

      <fieldset className="segmented-control">
        <legend>Resultattype</legend>
        <div>
          <button type="button" aria-pressed={scope === 'round'} onClick={() => onScopeChange('round')}>Runde</button>
          <button type="button" aria-pressed={scope === 'tournament'} onClick={() => onScopeChange('tournament')}>Turnering</button>
        </div>
      </fieldset>

      <fieldset className="segmented-control">
        <legend>Beregning</legend>
        <div>
          <button type="button" aria-pressed={metric === 'gross'} onClick={() => onMetricChange('gross')}>Brutto</button>
          <button type="button" aria-pressed={metric === 'net'} onClick={() => onMetricChange('net')}>Netto</button>
        </div>
      </fieldset>

      {scope === 'round' && (
        <label className="leaderboard-select">
          <span>Runde</span>
          <select
            value={roundId ?? ''}
            disabled={roundsPending || rounds.length === 0}
            onChange={(event) => onRoundChange(event.target.value)}
          >
            {rounds.length === 0 && <option value="">{roundsPending ? 'Laster runder …' : 'Ingen runder'}</option>}
            {rounds.map((round) => (
              <option key={round.id} value={round.id}>Runde {round.round_number}: {round.name}</option>
            ))}
          </select>
        </label>
      )}
    </section>
  )
}
