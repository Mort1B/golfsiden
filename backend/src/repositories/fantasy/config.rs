use super::*;
use chrono::{DateTime, Utc};

pub async fn configure(pool: &PgPool, session: Uuid, t: Uuid, on: bool) -> Result<Game, Error> {
    let (mut tx, actor) = begin(pool, session, t, true).await?;
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR UPDATE")
        .bind(t)
        .execute(&mut *tx)
        .await?;
    // Drain pre-enable roster/member transactions. Subsequent relevant writes
    // serialize with the round locks; inserts also serialize on the parent FK.
    sqlx::query("SELECT p.id FROM tournament_players tp JOIN players p ON p.id=tp.player_id WHERE tp.tournament_id=$1 ORDER BY p.id FOR SHARE OF tp,p").bind(t).fetch_all(&mut *tx).await?;
    sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 ORDER BY user_id FOR SHARE").bind(t).fetch_all(&mut *tx).await?;
    let closed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM rounds WHERE tournament_id=$1 AND status<>'draft') OR EXISTS(SELECT 1 FROM fantasy_rounds WHERE tournament_id=$1 AND (locked_at IS NOT NULL OR deadline<=clock_timestamp()))").bind(t).fetch_one(&mut *tx).await?;
    if closed {
        return Err(Error::Closed);
    }
    let game=sqlx::query_as("INSERT INTO fantasy_games(tournament_id,enabled) VALUES($1,$2) ON CONFLICT(tournament_id) DO UPDATE SET enabled=excluded.enabled RETURNING tournament_id,enabled,rules_version").bind(t).bind(on).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO fantasy_rounds(round_id,tournament_id) SELECT id,tournament_id FROM rounds WHERE tournament_id=$1 ON CONFLICT DO NOTHING").bind(t).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO fantasy_configuration_audits(tournament_id,actor_id,action) VALUES($1,$2,$3)",
    )
    .bind(t)
    .bind(actor)
    .bind(if on { "enabled" } else { "disabled" })
    .execute(&mut *tx)
    .await?;
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    Ok(game)
}
pub async fn enter(pool: &PgPool, session: Uuid, t: Uuid) -> Result<(), Error> {
    let (mut tx, actor) = begin(pool, session, t, false).await?;
    enabled(&mut tx, t).await?;
    sqlx::query(
        "INSERT INTO fantasy_entries(tournament_id,user_id) VALUES($1,$2) ON CONFLICT DO NOTHING",
    )
    .bind(t)
    .bind(actor)
    .execute(&mut *tx)
    .await?;
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn set_deadline(
    pool: &PgPool,
    session: Uuid,
    t: Uuid,
    r: Uuid,
    deadline: Option<DateTime<Utc>>,
) -> Result<Window, Error> {
    let (mut tx, actor) = begin(pool, session, t, true).await?;
    enabled(&mut tx, t).await?;
    let window = window(&mut tx, t, r).await?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    if window.locked_at.is_some()
        || window.opened_at.is_some()
        || window.deadline.is_some_and(|d| d <= now)
    {
        return Err(Error::Closed);
    }
    if deadline.is_some_and(|d| d <= now) {
        return Err(Error::Invalid);
    }
    let result=sqlx::query_as("UPDATE fantasy_rounds SET deadline=$2 WHERE round_id=$1 RETURNING round_id,deadline,opened_at,locked_at").bind(r).bind(deadline).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO fantasy_configuration_audits(tournament_id,actor_id,action,round_id,deadline) VALUES($1,$2,'deadline',$3,$4)").bind(t).bind(actor).bind(r).bind(deadline).execute(&mut *tx).await?;
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    Ok(result)
}
pub(super) async fn window(
    tx: &mut Transaction<'_, Postgres>,
    t: Uuid,
    r: Uuid,
) -> Result<Window, Error> {
    sqlx::query_as("SELECT round_id,deadline,opened_at,locked_at FROM fantasy_rounds WHERE tournament_id=$1 AND round_id=$2").bind(t).bind(r).fetch_optional(&mut **tx).await?.ok_or(AuthorizationError::NotFound.into())
}
pub async fn game(pool: &PgPool, session: Uuid, t: Uuid) -> Result<Option<Game>, Error> {
    let (mut tx, _) = begin(pool, session, t, false).await?;
    let result = sqlx::query_as(
        "SELECT tournament_id,enabled,rules_version FROM fantasy_games WHERE tournament_id=$1",
    )
    .bind(t)
    .fetch_optional(&mut *tx)
    .await?;
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    Ok(result)
}
