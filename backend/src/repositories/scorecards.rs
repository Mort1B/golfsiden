mod handicaps;
mod mutations;
mod rows;

use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    models::{RoundStatus, TournamentRole},
    score_visibility::{VisibilityFacts, visibility},
    scorecards::{
        ConfirmationState, ScoreEntry, ScoreOwner, ScorecardReadProjection, ScorecardSummary,
        read_projection, summarize,
    },
    scoring::ScoringError,
};
use crate::repositories::{auth, score_authorization};
use handicaps::validate_owner;
use rows::{
    ConfirmationRow, HoleScoreRow, ReadVisibilityContext, RoundContext, ScoreRow,
    hole_source_from_row, score_from_row,
};

pub use mutations::{
    AuthenticatedSaveScore, MutationResult, SaveScore, confirm, confirm_authenticated, save,
    save_authenticated,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScorecardConflict {
    RoundNotEditable,
    OwnerFormatMismatch,
    OwnerNotEligible,
    HoleMismatch,
    Incomplete,
}

impl ScorecardConflict {
    pub fn code(self) -> &'static str {
        match self {
            Self::RoundNotEditable => "round_not_editable",
            Self::OwnerFormatMismatch => "score_owner_format_mismatch",
            Self::OwnerNotEligible => "score_owner_not_eligible",
            Self::HoleMismatch => "score_hole_mismatch",
            Self::Incomplete => "scorecard_incomplete",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::RoundNotEditable => "scores require an open or completed round",
            Self::OwnerFormatMismatch => "score owner does not match the round format",
            Self::OwnerNotEligible => "score owner is not eligible for this round",
            Self::HoleMismatch => "hole does not belong to the round tee",
            Self::Incomplete => "every configured hole must be scored before confirmation",
        }
    }
}

#[derive(Debug, Error)]
pub enum ScorecardError {
    #[error("resource not found")]
    NotFound,
    #[error("authentication required")]
    Unauthenticated,
    #[error("score owner is not writable by this user")]
    Forbidden,
    #[error("scorecard conflict")]
    Conflict(ScorecardConflict),
    #[error("stored score data is invalid")]
    InvalidStoredData,
    #[error("score calculation failed")]
    Scoring(#[from] ScoringError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn get(
    pool: &PgPool,
    round_id: Uuid,
    owner: ScoreOwner,
) -> Result<ScorecardSummary, ScorecardError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *transaction)
        .await?;
    let context = load_round(&mut transaction, round_id, false).await?;
    let summary = build_summary(&mut transaction, &context, owner).await?;
    transaction.commit().await?;
    Ok(summary)
}

pub async fn get_read_authenticated(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    owner: ScoreOwner,
) -> Result<ScorecardReadProjection, ScorecardError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    let context = load_round(&mut transaction, round_id, false).await?;
    let principal = auth::lock_active_session(&mut transaction, session_id)
        .await?
        .ok_or(ScorecardError::Unauthenticated)?;
    let role = membership_role(&mut transaction, context.tournament_id, principal.user_id)
        .await?
        .ok_or(ScorecardError::Forbidden)?;
    let read_context = sqlx::query_as::<_, ReadVisibilityContext>(
        "SELECT r.round_number, t.number_of_rounds AS tournament_round_count, r.final_scores_hidden_until FROM rounds r JOIN tournaments t ON t.id = r.tournament_id WHERE r.id = $1",
    )
    .bind(round_id)
    .fetch_one(&mut *transaction)
    .await?;
    let observed_at = sqlx::query_scalar("SELECT transaction_timestamp()")
        .fetch_one(&mut *transaction)
        .await?;
    let metadata = visibility(VisibilityFacts {
        role,
        is_final_round: read_context.round_number == read_context.tournament_round_count,
        status: context.status,
        number_of_holes: context.number_of_holes,
        hidden_until: read_context.final_scores_hidden_until,
        observed_at,
    });
    let summary = build_summary(&mut transaction, &context, owner).await?;
    let result = read_projection(summary, metadata);
    transaction.commit().await?;
    Ok(result)
}

pub async fn get_scoring_authenticated(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    owner: ScoreOwner,
) -> Result<ScorecardSummary, ScorecardError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    let context = load_round(&mut transaction, round_id, false).await?;
    if !matches!(context.status, RoundStatus::Open | RoundStatus::Completed) {
        return Err(ScorecardError::Forbidden);
    }
    score_authorization::authorize_scoring_read(
        &mut transaction,
        session_id,
        context.tournament_id,
        round_id,
        context.scoring_format,
        owner,
    )
    .await
    .map_err(|error| match error {
        score_authorization::ScoreAuthorizationError::NotFound => ScorecardError::NotFound,
        score_authorization::ScoreAuthorizationError::Unauthenticated => {
            ScorecardError::Unauthenticated
        }
        score_authorization::ScoreAuthorizationError::Forbidden => ScorecardError::Forbidden,
        score_authorization::ScoreAuthorizationError::Database(error) => {
            ScorecardError::Database(error)
        }
    })?;
    let summary = build_summary(&mut transaction, &context, owner).await?;
    transaction.commit().await?;
    Ok(summary)
}

async fn membership_role(
    connection: &mut PgConnection,
    tournament_id: Uuid,
    user_id: Uuid,
) -> Result<Option<TournamentRole>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT role FROM tournament_memberships WHERE tournament_id = $1 AND user_id = $2 FOR SHARE",
    )
    .bind(tournament_id)
    .bind(user_id)
    .fetch_optional(connection)
    .await
}

async fn load_round(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
    lock: bool,
) -> Result<RoundContext, ScorecardError> {
    let sql = if lock {
        "SELECT id, tournament_id, tee_id, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format FROM rounds WHERE id = $1 FOR UPDATE"
    } else {
        "SELECT id, tournament_id, tee_id, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format FROM rounds WHERE id = $1"
    };
    sqlx::query_as(sql)
        .bind(round_id)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or(ScorecardError::NotFound)
}

async fn validate_hole(
    transaction: &mut Transaction<'_, Postgres>,
    context: &RoundContext,
    hole_id: Uuid,
) -> Result<(), ScorecardError> {
    let tee_id = sqlx::query_scalar::<_, Uuid>("SELECT tee_id FROM holes WHERE id = $1")
        .bind(hole_id)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or(ScorecardError::NotFound)?;
    if Some(tee_id) == context.tee_id {
        Ok(())
    } else {
        Err(ScorecardError::Conflict(ScorecardConflict::HoleMismatch))
    }
}

async fn load_score(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
    hole_id: Uuid,
    owner: ScoreOwner,
) -> Result<Option<ScoreEntry>, ScorecardError> {
    let row = sqlx::query_as::<_, ScoreRow>("SELECT id, round_id, hole_id, player_id, team_id, gross_strokes, submitted_by, submitted_at, updated_at FROM scores WHERE round_id = $1 AND hole_id = $2 AND (($3::uuid IS NOT NULL AND player_id = $3) OR ($4::uuid IS NOT NULL AND team_id = $4)) FOR UPDATE")
        .bind(round_id).bind(hole_id).bind(owner.player_id()).bind(owner.team_id())
        .fetch_optional(&mut **transaction).await?;
    row.map(score_from_row).transpose()
}

async fn count_scores(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
    owner: ScoreOwner,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM scores WHERE round_id = $1 AND (($2::uuid IS NOT NULL AND player_id = $2) OR ($3::uuid IS NOT NULL AND team_id = $3))")
        .bind(round_id).bind(owner.player_id()).bind(owner.team_id())
        .fetch_one(&mut **transaction).await
}

async fn load_confirmation(
    connection: &mut PgConnection,
    round_id: Uuid,
    owner: ScoreOwner,
) -> Result<Option<ConfirmationRow>, sqlx::Error> {
    sqlx::query_as("SELECT confirmed_by, confirmed_at FROM scorecard_confirmations WHERE round_id = $1 AND (($2::uuid IS NOT NULL AND player_id = $2) OR ($3::uuid IS NOT NULL AND team_id = $3))")
        .bind(round_id).bind(owner.player_id()).bind(owner.team_id())
        .fetch_optional(connection).await
}

async fn build_summary(
    connection: &mut PgConnection,
    context: &RoundContext,
    owner: ScoreOwner,
) -> Result<ScorecardSummary, ScorecardError> {
    let playing_handicap = validate_owner(connection, context, owner).await?;
    let rows = sqlx::query_as::<_, HoleScoreRow>("SELECT $1::uuid AS round_id, h.id AS hole_id, h.hole_number, h.par, h.stroke_index, s.id AS score_id, s.player_id, s.team_id, s.gross_strokes, s.submitted_by, s.submitted_at, s.updated_at FROM holes h LEFT JOIN scores s ON s.round_id = $1 AND s.hole_id = h.id AND (($2::uuid IS NOT NULL AND s.player_id = $2) OR ($3::uuid IS NOT NULL AND s.team_id = $3)) WHERE h.tee_id = $4 ORDER BY h.hole_number")
        .bind(context.id).bind(owner.player_id()).bind(owner.team_id()).bind(context.tee_id)
        .fetch_all(&mut *connection).await?;
    let sources = rows
        .into_iter()
        .map(hole_source_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    let confirmation = load_confirmation(connection, context.id, owner)
        .await?
        .map(|row| ConfirmationState {
            confirmed_by: row.confirmed_by,
            confirmed_at: row.confirmed_at,
        });
    summarize(
        context.id,
        owner,
        playing_handicap,
        context.number_of_holes,
        sources,
        confirmation,
    )
    .map_err(ScorecardError::from)
}
