import { Flag, Radio, Users } from 'lucide-react'
import { Link } from 'react-router-dom'
import type { LeaderboardMetric, Round, TournamentLeaderboard, TournamentLeaderboardEntry } from '../../api/types'
import {
  bestRoundsProgressLabel,
  mandatoryRoundDisplayState,
  mandatoryRoundProgressLabel,
  metricLabel,
  positionLabel,
  provisionalProgressLabel,
  scoreToParLabel,
  selectedProvisional,
} from './format'
import { validateMandatoryRound } from '../../api/mandatoryRounds'
import { playerHistoryUrl } from './drilldownRoutes'

function entryState(entry: TournamentLeaderboardEntry, requiredCount: number): string {
  const progress = bestRoundsProgressLabel(entry, requiredCount)
  return entry.status === 'withdrawn' ? `Trukket · ${progress}` : progress
}

function TournamentRow({
  entry,
  metric,
  hasCurrentRound,
  requiredCount,
  mandatoryRound,
  rounds,
  visibility,
  tournamentId,
}: {
  entry: TournamentLeaderboardEntry
  metric: LeaderboardMetric
  hasCurrentRound: boolean
  requiredCount: number
  mandatoryRound: Round | null
  rounds: Round[]
  visibility: TournamentLeaderboard['visibility']['mode']
  tournamentId: string
}) {
  const total = metric === 'gross' ? entry.gross_total : entry.net_total
  const hasSelectedScore = entry.contributions.some((contribution) => contribution.counted)
  const provisional = selectedProvisional(entry)
  const mandatory = entry.contributions.find((contribution) => contribution.mandatory)
  const mandatoryState = mandatoryRound === null
    ? null
    : mandatoryRoundDisplayState(mandatoryRound, rounds, visibility, mandatory)
  return (
    <li className={entry.status === 'withdrawn' ? 'withdrawn' : undefined}>
      <Link className="leaderboard-row leaderboard-row-link" to={playerHistoryUrl(tournamentId, entry.player_id, metric)}>
      <div
        className="leaderboard-position"
        aria-label={entry.position === null
          ? undefined
          : `${provisional === null ? '' : 'Foreløpig '}${entry.tied ? 'delt ' : ''}plass ${entry.position}`}
      >
        {positionLabel(entry.position, entry.tied)}
      </div>
      <div className="leaderboard-identity">
        <h2>{entry.display_name}</h2>
        <p className="leaderboard-progress">{entryState(entry, requiredCount)}</p>
        {provisional !== null && (
          <p className="leaderboard-live">
            {provisional.owner.type === 'team' ? <Users aria-hidden="true" /> : <Radio aria-hidden="true" />}
            {provisionalProgressLabel(provisional)}
          </p>
        )}
        {mandatoryRound !== null && mandatoryState !== null && (
          <p className="leaderboard-mandatory">
            <Flag aria-hidden="true" />
            {mandatoryRoundProgressLabel(
              mandatoryRound.name,
              mandatoryState,
              mandatory?.provisional === true
                ? { holesScored: mandatory.holes_scored, numberOfHoles: mandatory.number_of_holes }
                : null,
            )}
          </p>
        )}
      </div>
      <div className={`leaderboard-score${provisional === null ? '' : ' provisional'}`}>
        <strong>{hasSelectedScore ? scoreToParLabel(entry.score_to_par) : '–'}</strong>
        <span>{hasSelectedScore
          ? `${provisional === null ? '' : 'Foreløpig · '}${total} ${metricLabel(metric).toLowerCase()}`
          : hasCurrentRound ? 'Ingen score ennå' : 'Ingen score'}</span>
      </div>
      </Link>
    </li>
  )
}

export function TournamentStandings({ leaderboard, rounds }: { leaderboard: TournamentLeaderboard; rounds: Round[] }) {
  const mandatoryRound = validateMandatoryRound(
    leaderboard.mandatory_round_id,
    rounds,
    'resultatdata',
    'leaderboard.mandatory_round_id round identity',
  )
  return (
    <div className="standings-section">
      <div className="standings-heading">
        <div><p>Samlet</p><h2>{metricLabel(leaderboard.metric)} resultat</h2></div>
        <span>Beste {leaderboard.required_counted_rounds} av {rounds.length}</span>
      </div>
      <ol className="leaderboard-list" aria-label={`${metricLabel(leaderboard.metric)} resultat for turneringen`}>
        {leaderboard.entries.map((entry) => (
          <TournamentRow
            key={entry.player_id}
            entry={entry}
            metric={leaderboard.metric}
            hasCurrentRound={leaderboard.current_round_id !== null}
            requiredCount={leaderboard.required_counted_rounds}
            mandatoryRound={mandatoryRound}
            rounds={rounds}
            visibility={leaderboard.visibility.mode}
            tournamentId={leaderboard.tournament_id}
          />
        ))}
      </ol>
    </div>
  )
}
