use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{
        models::RoundStatus,
        scorecards::{ScoreEntry, ScoreOwner, ScorecardSummary},
    },
    repositories::score_authorization::{self, ScoreAuthorizationError},
};

use super::{
    ScorecardConflict, ScorecardError, build_summary, count_scores, load_confirmation, load_round,
    load_score, rows::RoundContext, rows::ScoreRow, rows::score_from_row, validate_hole,
    validate_owner,
};

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

pub struct AuthenticatedSaveScore {
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub owner: ScoreOwner,
    pub gross_strokes: i16,
    pub session_id: Uuid,
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
    let submitted_by = input.submitted_by;
    let result = save_with_actor(&mut transaction, &context, input, submitted_by).await?;
    transaction.commit().await?;
    Ok(result)
}

pub async fn save_authenticated(
    pool: &PgPool,
    input: AuthenticatedSaveScore,
) -> Result<MutationResult<ScoreEntry>, ScorecardError> {
    let mut transaction = pool.begin().await?;
    let context = load_round(&mut transaction, input.round_id, true).await?;
    require_editable(&context)?;
    validate_owner(&mut transaction, &context, input.owner).await?;
    validate_hole(&mut transaction, &context, input.hole_id).await?;
    let submitted_by = score_authorization::authorize_mutation(
        &mut transaction,
        input.session_id,
        context.id,
        context.scoring_format,
        input.owner,
    )
    .await
    .map_err(map_authorization_error)?;
    let result = save_with_actor(
        &mut transaction,
        &context,
        SaveScore {
            round_id: input.round_id,
            hole_id: input.hole_id,
            owner: input.owner,
            gross_strokes: input.gross_strokes,
            submitted_by,
        },
        submitted_by,
    )
    .await?;
    transaction.commit().await?;
    Ok(result)
}

async fn save_with_actor(
    transaction: &mut Transaction<'_, Postgres>,
    context: &RoundContext,
    input: SaveScore,
    submitted_by: Uuid,
) -> Result<MutationResult<ScoreEntry>, ScorecardError> {
    let existing = load_score(transaction, input.round_id, input.hole_id, input.owner).await?;
    if let Some(score) = existing.as_ref()
        && score.gross_strokes == input.gross_strokes
    {
        return Ok(MutationResult {
            value: score.clone(),
            changed: false,
        });
    }

    set_mutation_context(transaction, input.round_id).await?;
    let row = if let Some(existing) = existing {
        sqlx::query_as::<_, ScoreRow>(
            "UPDATE scores SET gross_strokes = $2, submitted_by = $3 WHERE id = $1 RETURNING id, round_id, hole_id, player_id, team_id, gross_strokes, submitted_by, submitted_at, updated_at",
        )
        .bind(existing.id)
        .bind(input.gross_strokes)
        .bind(submitted_by)
        .fetch_one(&mut **transaction)
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
        .bind(submitted_by)
        .fetch_one(&mut **transaction)
        .await?
    };
    Ok(MutationResult {
        value: score_from_row(row)?,
        changed: true,
    })
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
    let result = confirm_with_actor(&mut transaction, &context, owner, confirmed_by).await?;
    transaction.commit().await?;
    Ok(result)
}

pub async fn confirm_authenticated(
    pool: &PgPool,
    round_id: Uuid,
    owner: ScoreOwner,
    session_id: Uuid,
) -> Result<MutationResult<ScorecardSummary>, ScorecardError> {
    let mut transaction = pool.begin().await?;
    let context = load_round(&mut transaction, round_id, true).await?;
    require_editable(&context)?;
    validate_owner(&mut transaction, &context, owner).await?;
    let confirmed_by = score_authorization::authorize_mutation(
        &mut transaction,
        session_id,
        context.id,
        context.scoring_format,
        owner,
    )
    .await
    .map_err(map_authorization_error)?;
    let result = confirm_with_actor(&mut transaction, &context, owner, confirmed_by).await?;
    transaction.commit().await?;
    Ok(result)
}

async fn confirm_with_actor(
    transaction: &mut Transaction<'_, Postgres>,
    context: &RoundContext,
    owner: ScoreOwner,
    confirmed_by: Uuid,
) -> Result<MutationResult<ScorecardSummary>, ScorecardError> {
    let round_id = context.id;
    if count_scores(transaction, round_id, owner).await? != i64::from(context.number_of_holes) {
        return Err(ScorecardError::Conflict(ScorecardConflict::Incomplete));
    }
    let changed = load_confirmation(transaction, round_id, owner)
        .await?
        .is_none();
    if changed {
        set_mutation_context(transaction, round_id).await?;
        sqlx::query("INSERT INTO scorecard_confirmations (id, round_id, tournament_id, player_id, team_id, confirmed_by) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(Uuid::new_v4())
            .bind(round_id)
            .bind(context.tournament_id)
            .bind(owner.player_id())
            .bind(owner.team_id())
            .bind(confirmed_by)
            .execute(&mut **transaction)
            .await?;
    }
    Ok(MutationResult {
        value: build_summary(transaction, context, owner).await?,
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

fn map_authorization_error(error: ScoreAuthorizationError) -> ScorecardError {
    match error {
        ScoreAuthorizationError::NotFound => ScorecardError::NotFound,
        ScoreAuthorizationError::Unauthenticated => ScorecardError::Unauthenticated,
        ScoreAuthorizationError::Forbidden => ScorecardError::Forbidden,
        ScoreAuthorizationError::Database(error) => ScorecardError::Database(error),
    }
}
