import { contributionEquivalent } from '../../api/leaderboards/values'
import { Flag, Radio, Users } from 'lucide-react'
import { Link } from 'react-router-dom'
import type { LeaderboardMetric, Round, TournamentLeaderboard, TournamentLeaderboardEntry } from '../../api/types'
import { metricLabel, scoreToParLabel } from './format'
import { scorecardUrl } from './drilldownRoutes'
import { contributionStateLabels, mandatoryPlayerHistoryLabel, orderedPlayerContributions } from './playerHistory'
import { LIVE_RESULTS_EXPLANATION, MANDATORY_ROUND_EXPLANATION } from './resultExplanations'

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
        <span>Beste {leaderboard.required_counted_rounds} av {rounds.filter(r => r.scoring_format !== 'singles_match_play').length}</span>
      </div>
      <p className="history-qualification">
        Kvalifisering: {player.counted_contributions} av {leaderboard.required_counted_rounds} nødvendige fullførte runder ·{' '}
        {player.eligible ? 'Kvalifisert' : 'Ikke kvalifisert ennå'}
      </p>
      <p>{LIVE_RESULTS_EXPLANATION}</p>
      {mandatoryLabel && <p>{MANDATORY_ROUND_EXPLANATION}</p>}
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
  const selectedTotal = contribution.value ? metric === 'gross' ? contribution.value.gross_points : contribution.value.net_points : metric === 'gross' ? contribution.gross_total : contribution.net_total
  const states = contributionStateLabels(contribution)
  return (
    <li>
      <Link className="history-result-link" to={scorecardUrl(tournamentId, round.id, contribution.owner, metric)}>
        <div className="history-result-heading">
          <div><p>Runde {round.round_number}</p><h3>{round.name}</h3></div>
          <strong>{scoreToParLabel(contributionEquivalent(contribution, metric))}{contribution.value ? ' ekvivalent' : ''}</strong>
        </div>
        <p className="history-owner">
          {contribution.owner.type === 'team' ? <Users aria-hidden="true" /> : <Radio aria-hidden="true" />}
          {contribution.owner_name} · {contribution.owner.type === 'team' ? 'Lag' : 'Spiller'}
        </p>
        <p className="history-result-state">
          {contribution.mandatory && <Flag aria-hidden="true" />}{states.join(' · ')}
        </p>
        {contribution.value && <p>Sammenlagtekvivalent = {contribution.provisional ? '2 per løste hull' : '36'} minus poeng. Pickup gir +2; tomme hull bidrar ikke. Opprinnelige slag bevares.</p>}
        <dl className="history-totals">
          <div><dt>{contribution.value ? 'Bruttopoeng' : 'Brutto'}</dt><dd>{contribution.value?.gross_points ?? contribution.gross_total}</dd></div>
          <div><dt>{contribution.value ? 'Nettopoeng' : 'Netto'}</dt><dd>{contribution.value?.net_points ?? contribution.net_total}</dd></div>
          {contribution.value ? <><div><dt>Bruttoekvivalent</dt><dd>{scoreToParLabel(contribution.value.gross_equivalent)}</dd></div><div><dt>Nettoekvivalent</dt><dd>{scoreToParLabel(contribution.value.net_equivalent)}</dd></div><div><dt>Faktiske bruttoslag</dt><dd>{contribution.value.actual_gross_total ?? '–'}</dd></div><div><dt>Faktiske nettoslag</dt><dd>{contribution.value.actual_net_total ?? '–'}</dd></div></> : <div><dt>Par</dt><dd>{contribution.par_total}</dd></div>}
          <div><dt>{metricLabel(metric)}</dt><dd>{selectedTotal}</dd></div>
        </dl>
      </Link>
    </li>
  )
}
