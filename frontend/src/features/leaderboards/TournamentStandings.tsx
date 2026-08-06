import { Users } from 'lucide-react'
import type { LeaderboardMetric, TournamentLeaderboard, TournamentLeaderboardEntry } from '../../api/types'
import { metricLabel, positionLabel, roundsLabel } from './format'

function entryState(entry: TournamentLeaderboardEntry): string {
  const progress = entry.completed_rounds === 0 ? 'Ingen fullførte runder' : roundsLabel(entry.completed_rounds)
  return entry.status === 'withdrawn' ? `Trukket · ${progress}` : progress
}

function TournamentRow({
  entry,
  metric,
  hasCurrentRound,
}: {
  entry: TournamentLeaderboardEntry
  metric: LeaderboardMetric
  hasCurrentRound: boolean
}) {
  const total = metric === 'gross' ? entry.gross_total : entry.net_total
  return (
    <li className={`leaderboard-row${entry.status === 'withdrawn' ? ' withdrawn' : ''}`}>
      <div className="leaderboard-position" aria-label={entry.tied && entry.position !== null ? `Delt plass ${entry.position}` : undefined}>
        {positionLabel(entry.position, entry.tied)}
      </div>
      <div className="leaderboard-identity">
        <h2>{entry.display_name}</h2>
        <p className="leaderboard-progress">{entryState(entry)}</p>
        {hasCurrentRound && (
          <p className="leaderboard-members">
            <Users aria-hidden="true" />{entry.current_team?.team_name ?? 'Ikke satt i lag i aktiv runde'}
          </p>
        )}
      </div>
      <div className="leaderboard-score">
        <strong>{entry.completed_rounds > 0 ? total : '–'}</strong>
        <span>{entry.completed_rounds > 0 ? metricLabel(metric) : 'Ingen score'}</span>
      </div>
    </li>
  )
}

export function TournamentStandings({ leaderboard }: { leaderboard: TournamentLeaderboard }) {
  return (
    <div className="standings-section">
      <div className="standings-heading">
        <div><p>Samlet</p><h2>{metricLabel(leaderboard.metric)} resultat</h2></div>
        <span>{roundsLabel(leaderboard.included_round_ids.length)}</span>
      </div>
      <ol className="leaderboard-list" aria-label={`${metricLabel(leaderboard.metric)} resultat for turneringen`}>
        {leaderboard.entries.map((entry) => (
          <TournamentRow
            key={entry.player_id}
            entry={entry}
            metric={leaderboard.metric}
            hasCurrentRound={leaderboard.current_round_id !== null}
          />
        ))}
      </ol>
    </div>
  )
}
