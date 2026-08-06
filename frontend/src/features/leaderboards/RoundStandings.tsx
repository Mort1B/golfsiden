import { CheckCircle2, Clock3, Users } from 'lucide-react'
import type { LeaderboardMetric, RoundLeaderboard, RoundLeaderboardEntry } from '../../api/types'
import { metricLabel, positionLabel, scoreToParLabel } from './format'

function progressLabel(entry: RoundLeaderboardEntry): string {
  if (entry.holes_scored === 0) return `Ikke startet · 0 av ${entry.number_of_holes} hull`
  if (!entry.complete) return `Pågår · ${entry.holes_scored} av ${entry.number_of_holes} hull`
  if (entry.confirmed) return `Bekreftet · ${entry.number_of_holes} av ${entry.number_of_holes} hull`
  return `Fullført · venter på bekreftelse`
}

function RoundRow({ entry, metric }: { entry: RoundLeaderboardEntry; metric: LeaderboardMetric }) {
  const total = metric === 'gross' ? entry.gross_total : entry.net_total
  const hasScore = entry.holes_scored > 0
  return (
    <li className="leaderboard-row">
      <div className="leaderboard-position" aria-label={entry.tied && entry.position !== null ? `Delt plass ${entry.position}` : undefined}>
        {positionLabel(entry.position, entry.tied)}
      </div>
      <div className="leaderboard-identity">
        <h2>{entry.owner_name}</h2>
        {entry.owner.type === 'team' && entry.members.length > 0 && (
          <p className="leaderboard-members"><Users aria-hidden="true" />{entry.members.map((member) => member.display_name).join(' · ')}</p>
        )}
        <p className="leaderboard-progress">
          {entry.confirmed ? <CheckCircle2 aria-hidden="true" /> : <Clock3 aria-hidden="true" />}
          {progressLabel(entry)}
        </p>
      </div>
      <div className="leaderboard-score">
        <strong>{hasScore ? scoreToParLabel(entry.score_to_par) : '–'}</strong>
        <span>{hasScore ? `${total} ${metricLabel(metric).toLowerCase()}` : 'Ingen score'}</span>
      </div>
    </li>
  )
}

export function RoundStandings({ leaderboard }: { leaderboard: RoundLeaderboard }) {
  return (
    <div className="standings-section">
      <div className="standings-heading">
        <div>
          <p>{leaderboard.scoring_format === 'team_scramble' ? 'Lag-scramble' : 'Individuelt slagspill'}</p>
          <h2>{metricLabel(leaderboard.metric)} resultat</h2>
        </div>
        <span>{leaderboard.status === 'draft' ? 'Kladd' : leaderboard.status === 'open' ? 'Åpen' : leaderboard.status === 'locked' ? 'Låst' : 'Fullført'}</span>
      </div>
      <ol className="leaderboard-list" aria-label={`${metricLabel(leaderboard.metric)} resultat for runden`}>
        {leaderboard.entries.map((entry) => <RoundRow key={`${entry.owner.type}-${entry.owner.id}`} entry={entry} metric={leaderboard.metric} />)}
      </ol>
    </div>
  )
}
