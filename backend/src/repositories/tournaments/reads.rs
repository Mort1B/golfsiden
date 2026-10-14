use std::future::Future;

use chrono::NaiveDate;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use super::COLUMNS;
use crate::{
    domain::models::{MyTournament, ScoringMode, Tournament, TournamentRole, TournamentStatus},
    repositories::{
        auth,
        tournament_authorization::{self, AuthorizationError},
    },
};

#[derive(Debug, Error)]
pub enum TournamentReadError {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("private read temporarily unavailable")]
    Unavailable,
}

async fn begin(
    pool: &PgPool,
    session: Uuid,
) -> Result<(Transaction<'_, Postgres>, Uuid), TournamentReadError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    // Hold session/user authority before memberships, through all result loading.
    let user = auth::lock_active_session(&mut tx, session)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?
        .user_id;
    Ok((tx, user))
}

async fn finish(
    mut tx: Transaction<'_, Postgres>,
    session: Uuid,
) -> Result<(), TournamentReadError> {
    // Row locks serialize revocation and credential replacement; only a fresh
    // database wall-clock check can reject natural expiry during a later wait.
    auth::lock_active_session(&mut tx, session)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    tx.commit().await?;
    Ok(())
}

async fn lock_memberships(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
) -> Result<(), sqlx::Error> {
    // One ordered lock query for the full collection, never one query per trip.
    // The payload query uses this same snapshot, excluding later membership adds.
    sqlx::query("SELECT tournament_id FROM tournament_memberships WHERE user_id=$1 ORDER BY tournament_id FOR SHARE")
        .bind(user).fetch_all(&mut **tx).await?;
    Ok(())
}

pub async fn list_for_session(
    pool: &PgPool,
    session: Uuid,
) -> Result<Vec<Tournament>, TournamentReadError> {
    retry(|| async {
        let (mut tx, user) = begin(pool, session).await?;
        lock_memberships(&mut tx, user).await?;
        let rows = sqlx::query_as::<_, Tournament>(&format!(
            "SELECT t.{columns}
             FROM tournament_memberships tm
             JOIN tournaments t ON t.id = tm.tournament_id
             WHERE tm.user_id = $1
             ORDER BY t.start_date DESC, t.name, t.id",
            columns = COLUMNS.replace(", ", ", t.")
        ))
        .bind(user)
        .fetch_all(&mut *tx)
        .await?;
        finish(tx, session).await?;
        Ok(rows)
    })
    .await
}

pub async fn get_for_session(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
) -> Result<Tournament, TournamentReadError> {
    retry(|| async {
        let (mut tx, user) = begin(pool, session).await?;
        tournament_authorization::require_tournament_member_read(&mut tx, user, tournament).await?;
        let row = sqlx::query_as::<_, Tournament>(&format!(
            "SELECT {COLUMNS} FROM tournaments WHERE id=$1"
        ))
        .bind(tournament)
        .fetch_one(&mut *tx)
        .await?;
        finish(tx, session).await?;
        Ok(row)
    })
    .await
}

pub async fn list_mine_for_session(
    pool: &PgPool,
    session: Uuid,
) -> Result<Vec<MyTournament>, TournamentReadError> {
    retry(|| async {
        let (mut tx, user) = begin(pool, session).await?;
        lock_memberships(&mut tx, user).await?;
        let rows = sqlx::query_as::<_, MyTournamentRow>(
            "SELECT t.id, t.name, t.description, t.start_date, t.end_date,
                    t.number_of_rounds, t.counted_rounds, t.mandatory_round_id, t.tie_break_policy, t.status, t.scoring_mode, t.created_at,
                    t.updated_at, tm.role,
                    CASE WHEN tp.player_id IS NOT NULL THEN u.player_id END AS player_id
             FROM tournament_memberships tm
             JOIN tournaments t ON t.id = tm.tournament_id
             JOIN users u ON u.id = tm.user_id
             LEFT JOIN tournament_players tp
               ON tp.tournament_id = tm.tournament_id AND tp.player_id = u.player_id
             WHERE tm.user_id = $1
             ORDER BY t.start_date DESC, lower(t.name), t.id",
        ).bind(user).fetch_all(&mut *tx).await?;
        finish(tx, session).await?;
        Ok(rows.into_iter().map(MyTournamentRow::into_model).collect())
    }).await
}

fn serialization_failure(error: &TournamentReadError) -> bool {
    match error {
        TournamentReadError::Database(error)
        | TournamentReadError::Authorization(AuthorizationError::Database(error)) => error
            .as_database_error()
            .is_some_and(|error| error.code().as_deref() == Some("40001")),
        _ => false,
    }
}

async fn retry<T, F, Fut>(mut read: F) -> Result<T, TournamentReadError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, TournamentReadError>>,
{
    // Retry only complete reads: the failed snapshot's user, roles and payload
    // are discarded. A second conflict has the established generic 503 contract.
    match read().await {
        Err(error) if serialization_failure(&error) => {}
        result => return result,
    }
    match read().await {
        Err(error) if serialization_failure(&error) => Err(TournamentReadError::Unavailable),
        result => result,
    }
}
#[derive(FromRow)]
struct MyTournamentRow {
    id: Uuid,
    name: String,
    description: String,
    start_date: NaiveDate,
    end_date: NaiveDate,
    number_of_rounds: i16,
    counted_rounds: Option<i16>,
    mandatory_round_id: Option<Uuid>,
    tie_break_policy: crate::domain::models::TournamentTieBreakPolicy,
    status: TournamentStatus,
    scoring_mode: ScoringMode,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    role: TournamentRole,
    player_id: Option<Uuid>,
}

impl MyTournamentRow {
    fn into_model(self) -> MyTournament {
        MyTournament {
            tournament: Tournament {
                id: self.id,
                name: self.name,
                description: self.description,
                start_date: self.start_date,
                end_date: self.end_date,
                number_of_rounds: self.number_of_rounds,
                counted_rounds: self.counted_rounds,
                mandatory_round_id: self.mandatory_round_id,
                tie_break_policy: self.tie_break_policy,
                status: self.status,
                scoring_mode: self.scoring_mode,
                created_at: self.created_at,
                updated_at: self.updated_at,
            },
            role: self.role,
            player_id: self.player_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[cfg(feature = "database-tests")]
    #[sqlx::test(migrations = "../migrations")]
    async fn snapshot_conflicts_retry_once_for_direct_and_nested_errors(pool: PgPool) {
        for nested in [false, true] {
            let attempts = AtomicUsize::new(0);
            let result: Result<(), _> = retry(|| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                let error = sqlx::query("DO $$ BEGIN RAISE EXCEPTION 'test snapshot conflict' USING ERRCODE='40001'; END $$").execute(&pool).await.unwrap_err();
                Err(if nested { TournamentReadError::Authorization(AuthorizationError::Database(error)) } else { TournamentReadError::Database(error) })
            }).await;
            assert!(matches!(result, Err(TournamentReadError::Unavailable)));
            assert_eq!(attempts.load(Ordering::SeqCst), 2);
        }
    }

    #[tokio::test]
    async fn nonserialization_errors_are_never_retried() {
        let attempts = AtomicUsize::new(0);
        let result: Result<(), _> = retry(|| async {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err(TournamentReadError::Authorization(
                AuthorizationError::Forbidden,
            ))
        })
        .await;
        assert!(matches!(
            result,
            Err(TournamentReadError::Authorization(
                AuthorizationError::Forbidden
            ))
        ));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}
