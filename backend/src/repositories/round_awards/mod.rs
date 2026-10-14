mod types;
use crate::{
    domain::models::{RoundStatus, ScoringFormat},
    repositories::{
        auth,
        tournament_authorization::{self, AuthorizationError},
    },
};
use sqlx::{PgPool, Postgres, Transaction};
pub use types::*;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("invalid award command or participant")]
    Invalid,
    #[error("award revision changed")]
    Revision,
    #[error("request identity was already used for another command")]
    Request,
    #[error("round is not completed")]
    State,
    #[error("round format does not support net awards")]
    Format,
    #[error("explicit correction acknowledgement required")]
    Correction,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
const RECEIPT: &str = "request_id,round_id,award_type,player_id,previous_player_id,revision::text AS revision,reason,correction,actor_id,created_at";
async fn recheck(tx: &mut Transaction<'_, Postgres>, session: Uuid) -> Result<(), Error> {
    auth::lock_active_session(tx, session)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    Ok(())
}
async fn context(
    tx: &mut Transaction<'_, Postgres>,
    round: Uuid,
) -> Result<(RoundStatus, ScoringFormat), Error> {
    Ok(
        sqlx::query_as("SELECT status,scoring_format FROM rounds WHERE id=$1")
            .bind(round)
            .fetch_one(&mut **tx)
            .await?,
    )
}
pub async fn read(pool: &PgPool, session: Uuid, round: Uuid) -> Result<View, Error> {
    let mut tx = pool.begin().await?;
    // The same round lock gives admin reads one coherent slots/history snapshot.
    let (tournament, _) =
        tournament_authorization::require_round_admin(&mut tx, session, round).await?;
    let (status, format) = context(&mut tx, round).await?;
    let availability = if format == ScoringFormat::SinglesMatchPlay {
        "unsupported_format"
    } else if matches!(status, RoundStatus::Completed | RoundStatus::Locked) {
        "available"
    } else {
        "round_not_completed"
    };
    let eligible_players = sqlx::query_as("SELECT s.player_id,p.display_name FROM round_handicap_snapshots s JOIN players p ON p.id=s.player_id WHERE s.round_id=$1 ORDER BY lower(p.display_name),p.display_name,s.player_id").bind(round).fetch_all(&mut *tx).await?;
    let awards = sqlx::query_as("SELECT kind AS award_type,a.player_id,coalesce(a.revision,0)::text AS revision FROM unnest(enum_range(NULL::round_award_type)) kind LEFT JOIN round_awards a ON a.round_id=$1 AND a.award_type=kind ORDER BY kind").bind(round).fetch_all(&mut *tx).await?;
    let history = sqlx::query_as(&format!(
        "SELECT {RECEIPT} FROM round_award_history WHERE round_id=$1 ORDER BY created_at,request_id"
    ))
    .bind(round)
    .fetch_all(&mut *tx)
    .await?;
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    Ok(View {
        round_id: round,
        tournament_id: tournament,
        availability,
        correction_required: status == RoundStatus::Locked,
        eligible_players,
        awards,
        history,
    })
}
/// The boolean reports a new commit so exact replays emit no duplicate invalidation.
pub async fn write(
    pool: &PgPool,
    session: Uuid,
    round: Uuid,
    command: &Command,
) -> Result<(Uuid, Receipt, bool), Error> {
    let expected = command.revision().ok_or(Error::Invalid)?;
    let encoded = serde_json::to_value(command).map_err(|_| Error::Invalid)?;
    let mut tx = pool.begin().await?;
    let (tournament, actor) =
        tournament_authorization::require_round_admin(&mut tx, session, round).await?;
    recheck(&mut tx, session).await?;
    // Serialize a request UUID across different rounds as well as the round itself.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 38))")
        .bind(command.request_id.to_string())
        .execute(&mut *tx)
        .await?;
    recheck(&mut tx, session).await?;
    let prior: Option<(Uuid, Uuid, serde_json::Value)> = sqlx::query_as(
        "SELECT round_id,actor_id,command FROM round_award_history WHERE request_id=$1",
    )
    .bind(command.request_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((prior_round, prior_actor, prior_command)) = prior {
        if prior_round != round || prior_actor != actor || prior_command != encoded {
            return Err(Error::Request);
        }
        let receipt = sqlx::query_as(&format!(
            "SELECT {RECEIPT} FROM round_award_history WHERE request_id=$1"
        ))
        .bind(command.request_id)
        .fetch_one(&mut *tx)
        .await?;
        recheck(&mut tx, session).await?;
        tx.commit().await?;
        return Ok((tournament, receipt, false));
    }
    let (status, format) = context(&mut tx, round).await?;
    if format == ScoringFormat::SinglesMatchPlay {
        return Err(Error::Format);
    }
    if !matches!(status, RoundStatus::Completed | RoundStatus::Locked) {
        return Err(Error::State);
    }
    let previous: Option<(Option<Uuid>, i64)> = sqlx::query_as(
        "SELECT player_id,revision FROM round_awards WHERE round_id=$1 AND award_type=$2",
    )
    .bind(round)
    .bind(command.award_type)
    .fetch_optional(&mut *tx)
    .await?;
    let (previous_player, revision) = previous.unwrap_or((None, 0));
    if revision != expected {
        return Err(Error::Revision);
    }
    if (status == RoundStatus::Locked || revision > 0 || command.player_id.is_none())
        && !command.correction
    {
        return Err(Error::Correction);
    }
    if command.player_id.is_none() && previous_player.is_none() {
        return Err(Error::Invalid);
    }
    if let Some(player) = command.player_id {
        let eligible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM round_handicap_snapshots WHERE round_id=$1 AND player_id=$2)").bind(round).bind(player).fetch_one(&mut *tx).await?;
        if !eligible {
            return Err(Error::Invalid);
        }
    }
    sqlx::query("SELECT set_config('app.round_award_session_id',$1,true)")
        .bind(session.to_string())
        .execute(&mut *tx)
        .await?;
    let receipt = sqlx::query_as(&format!("INSERT INTO round_award_history(request_id,round_id,award_type,player_id,previous_player_id,revision,reason,correction,actor_id,command) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING {RECEIPT}"))
        .bind(command.request_id).bind(round).bind(command.award_type).bind(command.player_id).bind(previous_player).bind(revision+1).bind(&command.reason).bind(command.correction).bind(actor).bind(encoded).fetch_one(&mut *tx).await?;
    if revision == 0 {
        sqlx::query(
            "INSERT INTO round_awards(round_id,award_type,player_id,revision) VALUES($1,$2,$3,$4)",
        )
        .bind(round)
        .bind(command.award_type)
        .bind(command.player_id)
        .bind(revision + 1)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE round_awards SET player_id=$3,revision=$4 WHERE round_id=$1 AND award_type=$2",
        )
        .bind(round)
        .bind(command.award_type)
        .bind(command.player_id)
        .bind(revision + 1)
        .execute(&mut *tx)
        .await?;
    }
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    Ok((tournament, receipt, true))
}
