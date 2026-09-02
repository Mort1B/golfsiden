import { Flag, Radio, Users } from 'lucide-react'
import { Link } from 'react-router-dom'
import type { LeaderboardMetric, Round, TournamentLeaderboard, TournamentLeaderboardEntry } from '../../api/types'
import { metricLabel, scoreToParLabel } from './format'
import { scorecardUrl } from './drilldownRoutes'
import { contributionStateLabels, mandatoryPlayerHistoryLabel, orderedPlayerContributions } from './playerHistory'

interface PlayerHistoryProps {
  leaderboard: TournamentLeaderboard
  player: TournamentLeaderboardEntry
  rounds: Round[]
}

export function PlayerHistory({ leaderboard, player, rounds }: PlayerHistoryProps) {
  const contributions = orderedPlayerContributions(player.contributions, rounds)
  const mandatoryLabel = mandatoryPlayerHistoryLabel(
    leaderboard.mandatory_round_id,
    rounds,
    leaderboard.visibility.mode,
    player.contributions,
  )
  return (
    <div className="player-history">
      <div className="history-summary">
        <div><p>{metricLabel(leaderboard.metric)} resultat</p><h2>{player.display_name}</h2></div>
        <span>Beste {leaderboard.required_counted_rounds} av {rounds.length}</span>
      </div>
      <p className="history-qualification">
        {player.counted_contributions} av {leaderboard.required_counted_rounds} fullførte tellende ·{' '}
        {player.eligible ? 'Kvalifisert' : 'Ikke kvalifisert ennå'}
      </p>
      {mandatoryLabel && (
        <p className="history-mandatory"><Flag aria-hidden="true" />{mandatoryLabel}</p>
      )}
      {contributions.length === 0 ? (
        <p className="history-empty">Ingen synlige runderesultater ennå.</p>
      ) : (
        <ol className="history-list" aria-label={`Synlige bidrag for ${player.display_name}`}>
          {contributions.map(({ contribution, round }) => (
            <ContributionRow key={round.id} contribution={contribution} round={round} metric={leaderboard.metric} tournamentId={leaderboard.tournament_id} />
          ))}
        </ol>
      )}
    </div>
  )
}

function ContributionRow({ contribution, round, metric, tournamentId }: {
  contribution: TournamentLeaderboardEntry['contributions'][number]
  round: Round
  metric: LeaderboardMetric
  tournamentId: string
}) {
  const selectedTotal = metric === 'gross' ? contribution.gross_total : contribution.net_total
  const states = contributionStateLabels(contribution)
  return (
    <li>
      <Link className="history-result-link" to={scorecardUrl(tournamentId, round.id, contribution.owner, metric)}>
        <div className="history-result-heading">
          <div><p>Runde {round.round_number}</p><h3>{round.name}</h3></div>
          <strong>{scoreToParLabel(contribution.score_to_par)}</strong>
        </div>
        <p className="history-owner">
          {contribution.owner.type === 'team' ? <Users aria-hidden="true" /> : <Radio aria-hidden="true" />}
          {contribution.owner_name} · {contribution.owner.type === 'team' ? 'Lag' : 'Spiller'}
        </p>
        <p className="history-result-state">
          {contribution.mandatory && <Flag aria-hidden="true" />}{states.join(' · ')}
        </p>
        <dl className="history-totals">
          <div><dt>Brutto</dt><dd>{contribution.gross_total}</dd></div>
          <div><dt>Netto</dt><dd>{contribution.net_total}</dd></div>
          <div><dt>Par</dt><dd>{contribution.par_total}</dd></div>
          <div><dt>{metricLabel(metric)}</dt><dd>{selectedTotal}</dd></div>
        </dl>
      </Link>
    </li>
  )
}
