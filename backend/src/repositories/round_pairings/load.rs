use std::collections::HashMap;

use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::models::{ParticipantStatus, ScoringFormat},
    repositories::round_pairings::types::{
        PairingEntrant, PairingGroup, PairingMember, RoundPairingRow, RoundPairings,
    },
};

pub(super) async fn round(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
) -> Result<Option<RoundPairingRow>, sqlx::Error> {
    sqlx::query_as("SELECT id AS round_id, tournament_id, status, scoring_format, updated_at FROM rounds WHERE id = $1")
        .bind(round_id)
        .fetch_optional(&mut **transaction)
        .await
}

pub(super) async fn model(
    transaction: &mut Transaction<'_, Postgres>,
    round: RoundPairingRow,
) -> Result<RoundPairings, sqlx::Error> {
    let entrants = sqlx::query_as::<_, PairingEntrant>(
        "SELECT tp.player_id, p.display_name, tp.status, p.active AS player_active
         FROM tournament_players tp JOIN players p ON p.id = tp.player_id
         WHERE tp.tournament_id = $1
         ORDER BY CASE WHEN tp.status = 'active' AND p.active THEN 0 ELSE 1 END,
                  lower(p.display_name), tp.player_id",
    )
    .bind(round.tournament_id)
    .fetch_all(&mut **transaction)
    .await?;
    let (active_entrants, inactive_entrants) = entrants
        .into_iter()
        .partition(|entrant| entrant.status == ParticipantStatus::Active && entrant.player_active);
    let all_teams = groups(
        transaction,
        "teams",
        "team_memberships",
        "team_id",
        round.round_id,
    )
    .await?;
    let flights = groups(
        transaction,
        "flights",
        "flight_memberships",
        "flight_id",
        round.round_id,
    )
    .await?;
    let (teams, legacy_individual_groups) = if round.scoring_format == ScoringFormat::TeamScramble {
        (all_teams, Vec::new())
    } else {
        (Vec::new(), all_teams)
    };
    Ok(RoundPairings {
        round_id: round.round_id,
        tournament_id: round.tournament_id,
        status: round.status,
        scoring_format: round.scoring_format,
        updated_at: round.updated_at,
        active_entrants,
        inactive_entrants,
        teams,
        flights,
        legacy_individual_groups,
    })
}

async fn groups(
    transaction: &mut Transaction<'_, Postgres>,
    table: &str,
    membership_table: &str,
    membership_id: &str,
    round_id: Uuid,
) -> Result<Vec<PairingGroup>, sqlx::Error> {
    // These identifiers are module-owned constants, never request data.
    let query = format!(
        "SELECT id, name, starting_hole, tee_time, created_at, updated_at FROM {table} WHERE round_id = $1 ORDER BY lower(name), id"
    );
    let mut groups = sqlx::query_as::<_, PairingGroup>(&query)
        .bind(round_id)
        .fetch_all(&mut **transaction)
        .await?;
    let member_query = format!(
        "SELECT m.{membership_id} AS group_id, m.player_id, p.display_name, m.display_order FROM {membership_table} m JOIN players p ON p.id = m.player_id WHERE m.round_id = $1 ORDER BY m.{membership_id}, m.display_order NULLS LAST, lower(p.display_name), m.player_id"
    );
    let mut members_by_group: HashMap<Uuid, Vec<PairingMember>> = HashMap::new();
    for row in sqlx::query_as::<_, MembershipRow>(&member_query)
        .bind(round_id)
        .fetch_all(&mut **transaction)
        .await?
    {
        members_by_group
            .entry(row.group_id)
            .or_default()
            .push(PairingMember {
                player_id: row.player_id,
                display_name: row.display_name,
                display_order: row.display_order,
            });
    }
    for group in &mut groups {
        group.members = members_by_group.remove(&group.id).unwrap_or_default();
    }
    Ok(groups)
}

#[derive(FromRow)]
struct MembershipRow {
    group_id: Uuid,
    player_id: Uuid,
    display_name: String,
    display_order: Option<i16>,
}
