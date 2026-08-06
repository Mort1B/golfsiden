mod rows;

use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    models::{RoundStatus, ScoringFormat},
    scorecards::{ConfirmationState, ScoreEntry, ScoreOwner, ScorecardSummary, summarize},
    scoring::{ScoringError, scramble_playing_handicap},
};

use rows::{
    ConfirmationRow, HoleScoreRow, RoundContext, ScoreRow, hole_source_from_row, score_from_row,
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
    #[error("scorecard conflict")]
    Conflict(ScorecardConflict),
    #[error("stored score data is invalid")]
    InvalidStoredData,
    #[error("score calculation failed")]
    Scoring(#[from] ScoringError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub struct MutationResult<T> {
    pub value: T,
    pub changed: bool,
}

pub struct SaveScore {
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub owner: ScoreOwner,
    pub gross_strokes: i16,
    pub submitted_by: Uuid,
}

pub async fn save(
    pool: &PgPool,
    input: SaveScore,
) -> Result<MutationResult<ScoreEntry>, ScorecardError> {
    let mut transaction = pool.begin().await?;
    let context = load_round(&mut transaction, input.round_id, true).await?;
    require_editable(&context)?;
    require_user(&mut transaction, input.submitted_by).await?;
    validate_owner(&mut transaction, &context, input.owner).await?;
    validate_hole(&mut transaction, &context, input.hole_id).await?;
    let existing = load_score(&mut transaction, input.round_id, input.hole_id, input.owner).await?;
    if let Some(score) = existing.as_ref()
        && score.gross_strokes == input.gross_strokes
    {
        transaction.commit().await?;
        return Ok(MutationResult {
            value: score.clone(),
            changed: false,
        });
    }

    set_mutation_context(&mut transaction, input.round_id).await?;
    let row = if let Some(existing) = existing {
        sqlx::query_as::<_, ScoreRow>(
            "UPDATE scores SET gross_strokes = $2, submitted_by = $3 WHERE id = $1 RETURNING id, round_id, hole_id, player_id, team_id, gross_strokes, submitted_by, submitted_at, updated_at",
        )
        .bind(existing.id)
        .bind(input.gross_strokes)
        .bind(input.submitted_by)
        .fetch_one(&mut *transaction)
        .await?
    } else {
        sqlx::query_as::<_, ScoreRow>(
            "INSERT INTO scores (id, round_id, tournament_id, hole_id, player_id, team_id, gross_strokes, submitted_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id, round_id, hole_id, player_id, team_id, gross_strokes, submitted_by, submitted_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(input.round_id)
        .bind(context.tournament_id)
        .bind(input.hole_id)
        .bind(input.owner.player_id())
        .bind(input.owner.team_id())
        .bind(input.gross_strokes)
        .bind(input.submitted_by)
        .fetch_one(&mut *transaction)
        .await?
    };
    let score = score_from_row(row)?;
    transaction.commit().await?;
    Ok(MutationResult {
        value: score,
        changed: true,
    })
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

pub async fn confirm(
    pool: &PgPool,
    round_id: Uuid,
    owner: ScoreOwner,
    confirmed_by: Uuid,
) -> Result<MutationResult<ScorecardSummary>, ScorecardError> {
    let mut transaction = pool.begin().await?;
    let context = load_round(&mut transaction, round_id, true).await?;
    require_editable(&context)?;
    require_user(&mut transaction, confirmed_by).await?;
    validate_owner(&mut transaction, &context, owner).await?;
    let score_count = count_scores(&mut transaction, round_id, owner).await?;
    if score_count != i64::from(context.number_of_holes) {
        return Err(ScorecardError::Conflict(ScorecardConflict::Incomplete));
    }
    let existing = load_confirmation(&mut transaction, round_id, owner).await?;
    let changed = existing.is_none();
    if changed {
        set_mutation_context(&mut transaction, round_id).await?;
        sqlx::query("INSERT INTO scorecard_confirmations (id, round_id, tournament_id, player_id, team_id, confirmed_by) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(Uuid::new_v4())
            .bind(round_id)
            .bind(context.tournament_id)
            .bind(owner.player_id())
            .bind(owner.team_id())
            .bind(confirmed_by)
            .execute(&mut *transaction)
            .await?;
    }
    let summary = build_summary(&mut transaction, &context, owner).await?;
    transaction.commit().await?;
    Ok(MutationResult {
        value: summary,
        changed,
    })
}

fn require_editable(context: &RoundContext) -> Result<(), ScorecardError> {
    if matches!(context.status, RoundStatus::Open | RoundStatus::Completed) {
        Ok(())
    } else {
        Err(ScorecardError::Conflict(
            ScorecardConflict::RoundNotEditable,
        ))
    }
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

async fn require_user(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), ScorecardError> {
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(user_id)
        .fetch_one(&mut **transaction)
        .await?;
    if exists {
        Ok(())
    } else {
        Err(ScorecardError::NotFound)
    }
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

async fn validate_owner(
    connection: &mut PgConnection,
    context: &RoundContext,
    owner: ScoreOwner,
) -> Result<i32, ScorecardError> {
    match (context.scoring_format, owner) {
        (ScoringFormat::IndividualStrokePlay, ScoreOwner::Player { id }) => {
            let handicap = sqlx::query_scalar::<_, i16>("SELECT playing_handicap FROM round_handicap_snapshots WHERE round_id = $1 AND player_id = $2")
                .bind(context.id).bind(id).fetch_optional(&mut *connection).await?;
            if let Some(handicap) = handicap {
                return Ok(if context.handicap_enabled {
                    i32::from(handicap)
                } else {
                    0
                });
            }
            owner_missing_or_ineligible(connection, "players", id).await
        }
        (ScoringFormat::TeamScramble, ScoreOwner::Team { id }) => {
            let team_round =
                sqlx::query_scalar::<_, Uuid>("SELECT round_id FROM teams WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&mut *connection)
                    .await?;
            let Some(team_round) = team_round else {
                return Err(ScorecardError::NotFound);
            };
            if team_round != context.id {
                return Err(ScorecardError::Conflict(
                    ScorecardConflict::OwnerNotEligible,
                ));
            }
            let handicaps = sqlx::query_scalar::<_, i16>("SELECT rhs.course_handicap FROM team_memberships tm JOIN round_handicap_snapshots rhs ON rhs.round_id = tm.round_id AND rhs.player_id = tm.player_id WHERE tm.team_id = $1 ORDER BY rhs.course_handicap, rhs.player_id")
                .bind(id).fetch_all(&mut *connection).await?;
            if !context.handicap_enabled && handicaps.len() == 2 {
                return Ok(0);
            }
            scramble_playing_handicap(
                &handicaps.into_iter().map(i32::from).collect::<Vec<_>>(),
                context.handicap_allowance_percent,
            )
            .map_err(ScorecardError::from)
        }
        _ => Err(ScorecardError::Conflict(
            ScorecardConflict::OwnerFormatMismatch,
        )),
    }
}

async fn owner_missing_or_ineligible(
    connection: &mut PgConnection,
    table: &str,
    id: Uuid,
) -> Result<i32, ScorecardError> {
    let exists = match table {
        "players" => {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM players WHERE id = $1)")
                .bind(id)
                .fetch_one(connection)
                .await?
        }
        _ => false,
    };
    if exists {
        Err(ScorecardError::Conflict(
            ScorecardConflict::OwnerNotEligible,
        ))
    } else {
        Err(ScorecardError::NotFound)
    }
}

async fn set_mutation_context(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(round_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
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
    summarize(context.id, owner, playing_handicap, sources, confirmation)
        .map_err(ScorecardError::from)
}
