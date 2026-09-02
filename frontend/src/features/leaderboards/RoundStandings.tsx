import { CheckCircle2, Clock3, Users } from 'lucide-react'
import { Link } from 'react-router-dom'
import type { LeaderboardMetric, RoundLeaderboard, RoundLeaderboardEntry } from '../../api/types'
import { metricLabel, positionLabel, scoreToParLabel, scoringFormatLabel } from './format'
import { scorecardUrl } from './drilldownRoutes'

function progressLabel(entry: RoundLeaderboardEntry, visibleHoleCount: number): string {
  if (entry.complete === null) return `Synlig score · ${entry.holes_scored} av ${visibleHoleCount} hull`
  if (entry.holes_scored === 0) return `Ikke startet · 0 av ${entry.number_of_holes} hull`
  if (!entry.complete) return `Pågår · ${entry.holes_scored} av ${entry.number_of_holes} hull`
  if (entry.confirmed) return `Bekreftet · ${entry.number_of_holes} av ${entry.number_of_holes} hull`
  return `Fullført · venter på bekreftelse`
}

export function RoundStandings({ leaderboard }: { leaderboard: RoundLeaderboard }) {
  return (
    <div className="standings-section">
      <div className="standings-heading">
        <div>
          <p>{scoringFormatLabel(leaderboard.scoring_format)}</p>
          <h2>{metricLabel(leaderboard.metric)} resultat</h2>
        </div>
        <span>{leaderboard.status === 'draft' ? 'Kladd' : leaderboard.status === 'open' ? 'Åpen' : leaderboard.status === 'locked' ? 'Låst' : 'Fullført'}</span>
      </div>
      <ol className="leaderboard-list" aria-label={`${metricLabel(leaderboard.metric)} resultat for runden`}>
        {leaderboard.entries.map((entry) => (
          <RoundRowWithTarget key={`${entry.owner.type}-${entry.owner.id}`} entry={entry} leaderboard={leaderboard} />
        ))}
      </ol>
    </div>
  )
}

function RoundRowWithTarget({ entry, leaderboard }: { entry: RoundLeaderboardEntry; leaderboard: RoundLeaderboard }) {
  const metric = leaderboard.metric
  const visibleHoleCount = leaderboard.visible_hole_count
  const content = <RoundRowContent entry={entry} metric={metric} visibleHoleCount={visibleHoleCount} />
  return entry.holes_scored > 0 ? (
    <li>
      <Link className="leaderboard-row leaderboard-row-link" to={scorecardUrl(
        leaderboard.tournament_id, leaderboard.round_id, entry.owner, metric,
      )}>{content}</Link>
    </li>
  ) : <li className="leaderboard-row">{content}</li>
}

function RoundRowContent({ entry, metric, visibleHoleCount }: { entry: RoundLeaderboardEntry; metric: LeaderboardMetric; visibleHoleCount: number }) {
  const total = metric === 'gross' ? entry.gross_total : entry.net_total
  const hasScore = entry.holes_scored > 0
  return <>
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
        {progressLabel(entry, visibleHoleCount)}
      </p>
    </div>
    <div className="leaderboard-score">
      <strong>{hasScore ? scoreToParLabel(entry.score_to_par) : '–'}</strong>
      <span>{hasScore ? `${total} ${metricLabel(metric).toLowerCase()}` : 'Ingen score'}</span>
    </div>
  </>
}
