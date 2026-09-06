import { Link } from 'react-router-dom'
import type { PairingValidation, ReadinessIssueCode } from '../../../api/roundLifecycle'
import type { RoundCompletionValidation } from '../../../api/scorecards'
import type { Round } from '../../../api/types'
import { scorecardUrl } from '../../leaderboards/drilldownRoutes'
import { scoringSearch } from '../../scoring/selection'
import { readinessDestination, roundManagementUrl } from './lifecycleState'

const issueLabels: Record<ReadinessIssueCode, string> = {
  round_not_draft: 'Runden er ikke lenger i kladd. Oppdater status.',
  tournament_not_openable: 'Start turneringen før runden åpnes.',
  no_active_entrants: 'Minst én aktiv deltaker må være påmeldt.',
  missing_team_assignment: 'Deltakere mangler lag.',
  ineligible_team_assignment: 'Lag inneholder deltakere som ikke kan spille.',
  empty_team: 'Et lag mangler spillere.',
  invalid_scramble_team_size: 'Hvert scramblelag må ha nøyaktig to spillere.',
  invalid_foursomes_team_size: 'Hvert foursomeslag må ha nøyaktig to spillere.',
  missing_flight_assignment: 'Deltakere mangler flight.',
  ineligible_flight_assignment: 'Flighter inneholder deltakere som ikke kan spille.',
  empty_flight: 'En flight mangler spillere.',
  legacy_individual_groups_present: 'Eldre individuelle grupper må konverteres til flighter.',
  team_split_across_flights: 'Begge lagspillere må være i samme flight.',
  missing_course: 'Velg bane for runden.',
  missing_tee: 'Velg utslagssted for runden.',
  mismatched_course_tee: 'Bane og utslagssted stemmer ikke overens.',
  missing_handicap_ratings: 'Utslagsstedet mangler gyldig baneverdi eller slope.',
  invalid_hole_count: 'Banen må ha riktig antall hull.',
  invalid_hole_numbers: 'Hullnummereringen må være komplett og entydig.',
  invalid_stroke_indexes: 'Hullene må ha komplette og unike indeksverdier.',
}

function Details({ label, names }: { label: string; names: string[] }) {
  if (!names.length) return null
  return <div><h5>{label}</h5><ul>{names.map((name, index) => <li key={index}>{name}</li>)}</ul></div>
}

export function OpeningReadiness({ round, validation }: { round: Round; validation: PairingValidation }) {
  return <div className="round-readiness">
    <h4>Krav før åpning</h4>
    {validation.ready ? <p>Runden er klar til å åpnes.</p> : <ul className="round-readiness-issues">
      {validation.issues.map((issue, index) => <li key={`${issue.code}-${index}`}>
        <span>{issueLabels[issue.code]}</span>
        <Link to={roundManagementUrl(round.tournament_id, round.id, readinessDestination(issue.code))}>
          {readinessDestination(issue.code) === 'courses' ? 'Til baneoppsett'
            : readinessDestination(issue.code) === 'pairings' ? 'Til spillegrupper'
              : readinessDestination(issue.code) === 'invitations' ? 'Til invitasjoner' : 'Til turneringsstatus'}
        </Link>
      </li>)}
    </ul>}
    {!validation.ready && <div className="round-readiness-details">
      <Details label="Mangler lag" names={validation.missing_players.map((item) => item.display_name)} />
      <Details label="Kan ikke spille på lag" names={validation.ineligible_players.map((item) => item.display_name)} />
      <Details label="Mangler flight" names={validation.missing_flight_players.map((item) => item.display_name)} />
      <Details label="Kan ikke spille i flight" names={validation.ineligible_flight_players.map((item) => item.display_name)} />
      <Details label="Lagstørrelser" names={validation.team_sizes.filter((item) => item.player_count !== 2).map((item) => `${item.team_name}: ${item.player_count} spillere`)} />
      <Details label="Tomme flighter" names={validation.flight_sizes.filter((item) => item.player_count === 0).map((item) => item.flight_name)} />
      <Details label="Eldre grupper" names={validation.legacy_individual_groups.map((item) => item.team_name)} />
      <Details label="Lag fordelt på flere flighter" names={validation.split_teams.map((item) => item.team_name)} />
    </div>}
  </div>
}

export function CompletionReadiness({ round, validation }: { round: Round; validation: RoundCompletionValidation }) {
  if (validation.visibility.mode !== 'full') return <p role="alert">Full administratorkontroll er ikke tilgjengelig. Oppdater tilgangen.</p>
  return <div className="round-readiness">
    <h4>Scorekort før {round.status === 'completed' ? 'låsing' : 'fullføring'}</h4>
    <p>Alle nødvendige scorekort må være komplette og bekreftet. En korrigering krever ny bekreftelse.</p>
    {!validation.owners.length && <p>Runden har ingen nødvendige scorekort og kan ikke fullføres eller låses.</p>}
    <ul className="round-completion-cards">
      {validation.owners.map((item) => <li key={`${item.owner.type}:${item.owner.id}`}>
        <div><strong>{item.owner_name}</strong><span>{item.holes_scored} av {item.required_holes} hull · {item.confirmed ? 'Bekreftet' : item.complete ? 'Mangler bekreftelse' : 'Ufullstendig · ikke bekreftet'}</span></div>
        <div className="round-card-links">
          {(!item.complete || !item.confirmed) && <Link aria-label={`${item.complete ? 'Bekreft scorekort' : 'Fyll ut scorekort'} for ${item.owner_name}`} to={`/score?${scoringSearch({ tournamentId: round.tournament_id, roundId: round.id, owner: item.owner, view: 'summary' })}`}>
            {item.complete ? 'Bekreft scorekort' : 'Fyll ut scorekort'}
          </Link>}
          <Link aria-label={`Les scorekort for ${item.owner_name}`} to={scorecardUrl(round.tournament_id, round.id, item.owner, 'gross')}>Les scorekort</Link>
        </div>
      </li>)}
    </ul>
  </div>
}
