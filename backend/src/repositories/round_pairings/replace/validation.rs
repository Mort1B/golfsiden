use std::collections::{HashMap, HashSet};

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{
        round_formats::{RoundFormatPolicy, ScoreOwnerKind},
        round_pairings::{ReplacementCommand, members_by_flight},
    },
    repositories::round_pairings::{
        RoundPairingsError,
        types::{LegacyFacts, LegacyMemberFact, RoundPairingRow, StoredGroup},
    },
};

pub(super) async fn identities_and_roster(
    tx: &mut Transaction<'_, Postgres>,
    round: &RoundPairingRow,
    command: &ReplacementCommand,
) -> Result<(), RoundPairingsError> {
    let team_ids: Vec<_> = command.teams.iter().map(|team| team.id).collect();
    let flight_ids: Vec<_> = command.flights.iter().map(|flight| flight.id).collect();
    if team_ids.iter().any(|id| flight_ids.contains(id)) {
        return Err(RoundPairingsError::IdentityConflict);
    }
    let team_conflict = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM teams WHERE id = ANY($1) AND round_id <> $2)",
    )
    .bind(&team_ids)
    .bind(round.round_id)
    .fetch_one(&mut **tx)
    .await?;
    let flight_conflict = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM flights WHERE id = ANY($1) AND round_id <> $2)",
    )
    .bind(&flight_ids)
    .bind(round.round_id)
    .fetch_one(&mut **tx)
    .await?;
    let cross_type_conflict = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM teams WHERE id = ANY($1))
             OR EXISTS(SELECT 1 FROM flights WHERE id = ANY($2))",
    )
    .bind(&flight_ids)
    .bind(&team_ids)
    .fetch_one(&mut **tx)
    .await?;
    if team_conflict || flight_conflict || cross_type_conflict {
        return Err(RoundPairingsError::IdentityConflict);
    }

    let eligible: HashSet<Uuid> = sqlx::query_scalar(
        "SELECT tp.player_id FROM tournament_players tp
         JOIN players p ON p.id = tp.player_id
         WHERE tp.tournament_id = $1 AND tp.status = 'active' AND p.active",
    )
    .bind(round.tournament_id)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .collect();
    let team_players: HashSet<_> = command
        .teams
        .iter()
        .flat_map(|team| team.members.iter().map(|member| member.player_id))
        .collect();
    let flight_players: HashSet<_> = command
        .flights
        .iter()
        .flat_map(|flight| flight.members.iter().map(|member| member.player_id))
        .collect();
    let policy = RoundFormatPolicy::for_format(round.scoring_format);
    let correct = flight_players.is_subset(&eligible)
        && team_players.is_subset(&eligible)
        && (policy.owner_kind() == ScoreOwnerKind::Team || command.teams.is_empty());
    if !correct {
        return Err(RoundPairingsError::InvalidRoster);
    }
    Ok(())
}

pub(super) async fn legacy_and_schedules(
    tx: &mut Transaction<'_, Postgres>,
    round: &RoundPairingRow,
    command: &ReplacementCommand,
) -> Result<LegacyFacts, RoundPairingsError> {
    let stored = sqlx::query_as::<_, StoredGroup>(
        "SELECT id, name, starting_hole, tee_time, created_at, updated_at FROM teams WHERE round_id = $1 ORDER BY id",
    )
    .bind(round.round_id)
    .fetch_all(&mut **tx)
    .await?;
    if RoundFormatPolicy::for_format(round.scoring_format).owner_kind() == ScoreOwnerKind::Team {
        if !command.legacy_conversions.is_empty() {
            return Err(RoundPairingsError::InvalidLegacyConversion);
        }
        validate_team_schedules(&stored, command)?;
        return Ok(LegacyFacts {
            by_flight: HashMap::new(),
            members_by_flight: HashMap::new(),
        });
    }
    if stored.len() != command.legacy_conversions.len() {
        return Err(RoundPairingsError::LegacyMappingRequired);
    }
    let flights_by_id: HashMap<_, _> = command
        .flights
        .iter()
        .map(|flight| (flight.id, flight))
        .collect();
    let mapped_flight_ids: Vec<_> = command
        .legacy_conversions
        .iter()
        .map(|mapping| mapping.flight_id)
        .collect();
    if sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM flights WHERE id = ANY($1))")
        .bind(&mapped_flight_ids)
        .fetch_one(&mut **tx)
        .await?
    {
        return Err(RoundPairingsError::IdentityConflict);
    }
    let mut by_flight = HashMap::new();
    let mut members_by_flight = HashMap::new();
    for group in stored {
        let mapping = command
            .legacy_conversions
            .iter()
            .find(|mapping| mapping.team_id == group.id)
            .ok_or(RoundPairingsError::LegacyMappingRequired)?;
        let flight = flights_by_id
            .get(&mapping.flight_id)
            .ok_or(RoundPairingsError::InvalidLegacyConversion)?;
        let members: Vec<LegacyMemberFact> = sqlx::query_as(
            "SELECT tm.player_id, tm.display_order, tm.created_at FROM team_memberships tm JOIN players p ON p.id=tm.player_id WHERE tm.team_id = $1 ORDER BY tm.display_order NULLS LAST, lower(p.display_name), tm.player_id",
        )
        .bind(group.id)
        .fetch_all(&mut **tx)
        .await?;
        let requested: Vec<_> = flight
            .members
            .iter()
            .map(|member| member.player_id)
            .collect();
        if flight.name != group.name
            || flight.starting_hole != group.starting_hole
            || flight.tee_time != group.tee_time
            || requested
                != members
                    .iter()
                    .map(|member| member.player_id)
                    .collect::<Vec<_>>()
        {
            return Err(RoundPairingsError::InvalidLegacyConversion);
        }
        members_by_flight.insert(
            mapping.flight_id,
            members
                .into_iter()
                .map(|member| (member.player_id, member))
                .collect(),
        );
        by_flight.insert(mapping.flight_id, group);
    }
    Ok(LegacyFacts {
        by_flight,
        members_by_flight,
    })
}

fn validate_team_schedules(
    stored: &[StoredGroup],
    command: &ReplacementCommand,
) -> Result<(), RoundPairingsError> {
    let flights = members_by_flight(command);
    for team in &command.teams {
        let Some(existing) = stored.iter().find(|stored| stored.id == team.id) else {
            if team.schedule_flight_id.is_some() {
                return Err(RoundPairingsError::InvalidScheduleTransfer);
            }
            continue;
        };
        let scheduled = existing.starting_hole.is_some() || existing.tee_time.is_some();
        if !scheduled && team.schedule_flight_id.is_some() {
            return Err(RoundPairingsError::InvalidScheduleTransfer);
        }
        if !scheduled {
            continue;
        }
        let flight_id = team
            .schedule_flight_id
            .ok_or(RoundPairingsError::InvalidScheduleTransfer)?;
        let flight = command
            .flights
            .iter()
            .find(|flight| flight.id == flight_id)
            .ok_or(RoundPairingsError::InvalidScheduleTransfer)?;
        let retained_members: HashSet<Uuid> =
            team.members.iter().map(|member| member.player_id).collect();
        if flights.get(&flight_id) != Some(&retained_members)
            || flight.starting_hole != existing.starting_hole
            || flight.tee_time != existing.tee_time
        {
            return Err(RoundPairingsError::InvalidScheduleTransfer);
        }
    }
    Ok(())
}

pub(super) async fn reject_referenced_team_deletions(
    tx: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
    command: &ReplacementCommand,
) -> Result<(), RoundPairingsError> {
    let retained: Vec<_> = command.teams.iter().map(|team| team.id).collect();
    let referenced = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM teams t
            WHERE t.round_id = $1 AND NOT (t.id = ANY($2))
              AND (EXISTS(SELECT 1 FROM scores s WHERE s.team_id = t.id)
                   OR EXISTS(SELECT 1 FROM scorecard_confirmations c WHERE c.team_id = t.id)))",
    )
    .bind(round_id)
    .bind(&retained)
    .fetch_one(&mut **tx)
    .await?;
    if referenced {
        return Err(RoundPairingsError::ReferencedTeam);
    }
    Ok(())
}
