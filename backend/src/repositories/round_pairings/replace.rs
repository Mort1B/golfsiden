use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{
        models::{RoundStatus, ScoringFormat},
        round_pairings::{ReplacementCommand, members_by_flight},
    },
    repositories::{
        round_pairings::{
            RoundPairingsError, load,
            types::{LegacyFacts, LegacyMemberFact, RoundPairings, StoredGroup},
        },
        tournament_authorization,
    },
};

pub(super) async fn execute(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    expected_updated_at: DateTime<Utc>,
    command: &ReplacementCommand,
) -> Result<RoundPairings, RoundPairingsError> {
    let mut tx = pool.begin().await?;
    tournament_authorization::require_round_admin(&mut tx, session_id, round_id).await?;
    let round = load::round(&mut tx, round_id)
        .await?
        .ok_or(tournament_authorization::AuthorizationError::NotFound)?;
    if round.status != RoundStatus::Draft {
        return Err(RoundPairingsError::NotDraft);
    }
    if round.updated_at != expected_updated_at {
        return Err(RoundPairingsError::Stale);
    }
    validate_identities_and_roster(&mut tx, &round, command).await?;
    let legacy = validate_legacy_and_schedules(&mut tx, &round, command).await?;
    reject_referenced_team_deletions(&mut tx, round_id, command, &legacy).await?;

    replace_memberships_and_groups(&mut tx, &round, command, legacy).await?;
    sqlx::query("UPDATE rounds SET updated_at = clock_timestamp() WHERE id = $1")
        .bind(round_id)
        .execute(&mut *tx)
        .await?;
    let updated = load::round(&mut tx, round_id)
        .await?
        .ok_or(tournament_authorization::AuthorizationError::NotFound)?;
    let model = load::model(&mut tx, updated).await?;
    tx.commit().await?;
    Ok(model)
}

async fn validate_identities_and_roster(
    tx: &mut Transaction<'_, Postgres>,
    round: &crate::repositories::round_pairings::types::RoundPairingRow,
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
    let correct = flight_players.is_subset(&eligible)
        && team_players.is_subset(&eligible)
        && match round.scoring_format {
            ScoringFormat::IndividualStrokePlay => command.teams.is_empty(),
            ScoringFormat::TeamScramble => true,
        };
    if !correct {
        return Err(RoundPairingsError::InvalidRoster);
    }
    Ok(())
}

async fn validate_legacy_and_schedules(
    tx: &mut Transaction<'_, Postgres>,
    round: &crate::repositories::round_pairings::types::RoundPairingRow,
    command: &ReplacementCommand,
) -> Result<LegacyFacts, RoundPairingsError> {
    let stored = sqlx::query_as::<_, StoredGroup>(
        "SELECT id, name, starting_hole, tee_time, created_at, updated_at FROM teams WHERE round_id = $1 ORDER BY id",
    )
    .bind(round.round_id)
    .fetch_all(&mut **tx)
    .await?;
    if round.scoring_format == ScoringFormat::TeamScramble {
        if !command.legacy_conversions.is_empty() {
            return Err(RoundPairingsError::InvalidLegacyConversion);
        }
        validate_scramble_schedules(&stored, command)?;
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

fn validate_scramble_schedules(
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

async fn reject_referenced_team_deletions(
    tx: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
    command: &ReplacementCommand,
    _legacy: &LegacyFacts,
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

async fn replace_memberships_and_groups(
    tx: &mut Transaction<'_, Postgres>,
    round: &crate::repositories::round_pairings::types::RoundPairingRow,
    command: &ReplacementCommand,
    legacy: LegacyFacts,
) -> Result<(), RoundPairingsError> {
    let existing_flight_members: HashMap<(Uuid, Uuid), DateTime<Utc>> =
        sqlx::query_as::<_, (Uuid, Uuid, DateTime<Utc>)>(
            "SELECT flight_id, player_id, created_at FROM flight_memberships WHERE round_id = $1",
        )
        .bind(round.round_id)
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(|(f, p, c)| ((f, p), c))
        .collect();
    let legacy_member_times: HashMap<(Uuid, Uuid), DateTime<Utc>> =
        sqlx::query_as::<_, (Uuid, Uuid, DateTime<Utc>)>(
            "SELECT team_id, player_id, created_at FROM team_memberships WHERE round_id = $1",
        )
        .bind(round.round_id)
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(|(t, p, c)| ((t, p), c))
        .collect();
    let existing_team_members = legacy_member_times.clone();

    sqlx::query("DELETE FROM flight_memberships WHERE round_id = $1")
        .bind(round.round_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM team_memberships WHERE round_id = $1")
        .bind(round.round_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM flights WHERE round_id = $1 AND NOT (id = ANY($2))")
        .bind(round.round_id)
        .bind(
            command
                .flights
                .iter()
                .map(|flight| flight.id)
                .collect::<Vec<_>>(),
        )
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM teams WHERE round_id = $1 AND NOT (id = ANY($2))")
        .bind(round.round_id)
        .bind(command.teams.iter().map(|team| team.id).collect::<Vec<_>>())
        .execute(&mut **tx)
        .await?;

    for team in &command.teams {
        sqlx::query(
            "INSERT INTO teams (id, round_id, tournament_id, name, starting_hole, tee_time)
             VALUES ($1, $2, $3, $4, NULL, NULL)
             ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, starting_hole = NULL, tee_time = NULL
             WHERE teams.name IS DISTINCT FROM EXCLUDED.name
                OR teams.starting_hole IS NOT NULL OR teams.tee_time IS NOT NULL",
        ).bind(team.id).bind(round.round_id).bind(round.tournament_id).bind(&team.name).execute(&mut **tx).await?;
        for (index, member) in team.members.iter().enumerate() {
            sqlx::query("INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id, display_order, created_at) VALUES ($1, $2, $3, $4, $5, COALESCE($6, now()))")
                .bind(team.id).bind(round.round_id).bind(round.tournament_id).bind(member.player_id)
                .bind(i16::try_from(index).map_err(|_| RoundPairingsError::InvalidRoster)?)
                .bind(existing_team_members.get(&(team.id, member.player_id)).copied())
                .execute(&mut **tx).await?;
        }
    }
    for flight in &command.flights {
        if let Some(group) = legacy.by_flight.get(&flight.id) {
            sqlx::query("INSERT INTO flights (id, round_id, tournament_id, name, starting_hole, tee_time, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
                .bind(flight.id).bind(round.round_id).bind(round.tournament_id).bind(&flight.name)
                .bind(flight.starting_hole).bind(flight.tee_time).bind(group.created_at).bind(group.updated_at)
                .execute(&mut **tx).await?;
        } else {
            sqlx::query("INSERT INTO flights (id, round_id, tournament_id, name, starting_hole, tee_time) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (id) DO UPDATE SET name=EXCLUDED.name, starting_hole=EXCLUDED.starting_hole, tee_time=EXCLUDED.tee_time WHERE flights.name IS DISTINCT FROM EXCLUDED.name OR flights.starting_hole IS DISTINCT FROM EXCLUDED.starting_hole OR flights.tee_time IS DISTINCT FROM EXCLUDED.tee_time")
                .bind(flight.id).bind(round.round_id).bind(round.tournament_id).bind(&flight.name)
                .bind(flight.starting_hole).bind(flight.tee_time).execute(&mut **tx).await?;
        }
        for (index, member) in flight.members.iter().enumerate() {
            let legacy_member = legacy
                .members_by_flight
                .get(&flight.id)
                .and_then(|members| members.get(&member.player_id));
            let created_at = existing_flight_members
                .get(&(flight.id, member.player_id))
                .copied()
                .or_else(|| legacy_member.map(|member| member.created_at));
            let display_order = match legacy_member {
                Some(member) => member.display_order,
                None => Some(i16::try_from(index).map_err(|_| RoundPairingsError::InvalidRoster)?),
            };
            sqlx::query("INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id, display_order, created_at) VALUES ($1,$2,$3,$4,$5,COALESCE($6, now()))")
                .bind(flight.id).bind(round.round_id).bind(round.tournament_id).bind(member.player_id)
                .bind(display_order)
                .bind(created_at).execute(&mut **tx).await?;
        }
    }
    Ok(())
}
