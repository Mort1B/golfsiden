use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::round_pairings::ReplacementCommand,
    repositories::round_pairings::{
        RoundPairingsError,
        types::{LegacyFacts, RoundPairingRow},
    },
};

pub(super) async fn replace_memberships_and_groups(
    tx: &mut Transaction<'_, Postgres>,
    round: &RoundPairingRow,
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
    let existing_team_members: HashMap<(Uuid, Uuid), DateTime<Utc>> =
        sqlx::query_as::<_, (Uuid, Uuid, DateTime<Utc>)>(
            "SELECT team_id, player_id, created_at FROM team_memberships WHERE round_id = $1",
        )
        .bind(round.round_id)
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(|(t, p, c)| ((t, p), c))
        .collect();

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
