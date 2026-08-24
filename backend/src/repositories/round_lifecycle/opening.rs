use std::collections::HashMap;

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{OpenRoundError, load};
use crate::domain::{
    handicap::{calculate, course_handicap_numerator, effective_index_tenths},
    models::{
        OpenRoundResult, ParticipantStatus, Round, RoundHandicapSnapshot, RoundTeamHandicapSnapshot,
    },
    round_formats::RoundFormatPolicy,
    round_lifecycle::validate,
    scoring::foursomes_playing_handicap,
};

const ROUND_COLUMNS: &str = "id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format, created_at, updated_at";

pub(super) async fn in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
) -> Result<OpenRoundResult, OpenRoundError> {
    let loaded = load::round(transaction, round_id, true)
        .await?
        .ok_or(OpenRoundError::NotFound)?;
    let validation = validate(&loaded.facts);
    if !validation.ready {
        return Err(OpenRoundError::NotReady(validation));
    }
    sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
        .bind(round_id)
        .execute(&mut **transaction)
        .await?;
    let slope_rating = loaded.facts.configuration.slope_rating.unwrap_or(113);
    let course_rating_tenths = loaded
        .facts
        .configuration
        .course_rating_tenths
        .unwrap_or(i32::from(loaded.course_par) * 10);
    let mut course_numerators = HashMap::new();
    for entrant in loaded.facts.entrants.iter().filter(|entrant| {
        entrant.participant_status == ParticipantStatus::Active && entrant.player_active
    }) {
        let effective_index_tenths =
            effective_index_tenths(loaded.facts.scoring_format, entrant.handicap_index_tenths);
        let handicap = calculate(
            effective_index_tenths,
            slope_rating,
            course_rating_tenths,
            loaded.course_par,
            loaded.handicap_allowance_percent,
            loaded.facts.handicap_enabled,
            loaded.facts.scoring_format,
        );
        course_numerators.insert(
            entrant.player_id,
            course_handicap_numerator(
                effective_index_tenths,
                slope_rating,
                course_rating_tenths,
                loaded.course_par,
            ),
        );
        sqlx::query("INSERT INTO round_handicap_snapshots (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap) VALUES ($1, $2, $3, $4::numeric / 10, $5, $6)")
            .bind(round_id).bind(loaded.tournament_id).bind(entrant.player_id)
            .bind(effective_index_tenths).bind(handicap.course_handicap).bind(handicap.playing_handicap)
            .execute(&mut **transaction).await?;
    }
    if RoundFormatPolicy::for_format(loaded.facts.scoring_format)
        .requires_preserved_team_handicap_snapshot()
    {
        for team in &loaded.facts.teams {
            let numerators = team
                .player_ids
                .iter()
                .filter_map(|player_id| course_numerators.get(player_id).copied())
                .collect::<Vec<_>>();
            let playing_handicap = if loaded.facts.handicap_enabled {
                foursomes_playing_handicap(&numerators)?
            } else {
                0
            };
            sqlx::query("INSERT INTO round_team_handicap_snapshots (round_id, tournament_id, team_id, playing_handicap) VALUES ($1, $2, $3, $4)")
                .bind(round_id)
                .bind(loaded.tournament_id)
                .bind(team.team_id)
                .bind(playing_handicap)
                .execute(&mut **transaction)
                .await?;
        }
    }
    sqlx::query("UPDATE rounds SET status='open' WHERE id=$1 AND status='draft'")
        .bind(round_id)
        .execute(&mut **transaction)
        .await?;
    let round =
        sqlx::query_as::<_, Round>(&format!("SELECT {ROUND_COLUMNS} FROM rounds WHERE id=$1"))
            .bind(round_id)
            .fetch_one(&mut **transaction)
            .await?;
    let handicap_snapshots = sqlx::query_as::<_, RoundHandicapSnapshot>(
        "SELECT round_id, tournament_id, player_id, handicap_index::float8 AS handicap_index, course_handicap, playing_handicap, captured_at FROM round_handicap_snapshots WHERE round_id=$1 ORDER BY player_id",
    ).bind(round_id).fetch_all(&mut **transaction).await?;
    let team_handicap_snapshots = sqlx::query_as::<_, RoundTeamHandicapSnapshot>(
        "SELECT round_id, tournament_id, team_id, playing_handicap, captured_at FROM round_team_handicap_snapshots WHERE round_id=$1 ORDER BY team_id",
    ).bind(round_id).fetch_all(&mut **transaction).await?;
    Ok(OpenRoundResult {
        round,
        handicap_snapshots,
        team_handicap_snapshots,
    })
}
