use super::*;
use serde::Serialize;
#[derive(Serialize)]
#[serde(untagged)]
pub enum TournamentResponse {
    Applicable(TournamentLeaderboard),
    NotApplicable {
        r#type: &'static str,
        reason: &'static str,
        tournament_id: Uuid,
        metric: LeaderboardMetric,
    },
}
/// Authorization, unavailable detection and any result assembly share one snapshot.
pub async fn tournament_response_for_member(
    pool: &PgPool,
    user_id: Uuid,
    tournament_id: Uuid,
    metric: LeaderboardMetric,
) -> Result<TournamentResponse, LeaderboardError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    tournament_authorization::require_tournament_member_read(&mut tx, user_id, tournament_id)
        .await?;
    let counted =
        sqlx::query_scalar::<_, Option<i16>>("SELECT counted_rounds FROM tournaments WHERE id=$1")
            .bind(tournament_id)
            .fetch_one(&mut *tx)
            .await?;
    let result = if counted.is_none() {
        TournamentResponse::NotApplicable {
            r#type: "not_applicable",
            reason: "match_only",
            tournament_id,
            metric,
        }
    } else {
        TournamentResponse::Applicable(
            tournament::assemble(
                &mut tx,
                tournament_id,
                metric,
                tournament::TournamentProjection::Member(user_id),
            )
            .await?,
        )
    };
    tx.commit().await?;
    Ok(result)
}
