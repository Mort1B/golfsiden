use super::{
    MutationResult, SaveScore, ScorecardConflict, ScorecardError, load_round, load_score,
    mutations, validate_hole, validate_owner,
};
use crate::{
    domain::scorecards::{
        AppliedScore, ExpectedScore, ScoreAcknowledgement, ScoreOwner, ScoreRevision,
    },
    repositories::{auth, score_authorization},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, Serialize)]
pub struct ConditionalSaveScore {
    pub request_id: Uuid,
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub owner: ScoreOwner,
    pub gross_strokes: i16,
    pub expected_score: ExpectedScore,
    #[serde(skip)]
    pub session_id: Uuid,
}
#[derive(FromRow)]
struct Receipt {
    request_hash: Vec<u8>,
    applied_score_id: Uuid,
    applied_revision: i64,
}

pub async fn save_conditional(
    pool: &PgPool,
    input: ConditionalSaveScore,
) -> Result<MutationResult<ScoreAcknowledgement>, ScorecardError> {
    let hash =
        Sha256::digest(serde_json::to_vec(&input).map_err(|_| ScorecardError::InvalidStoredData)?)
            .to_vec();
    let mut tx = pool.begin().await?;
    // Keep the existing round -> current authority -> score lock order. Every
    // receipt replay crosses the same authorization and lifecycle boundary.
    let context = load_round(&mut tx, input.round_id, true).await?;
    mutations::require_editable(&context)?;
    validate_owner(&mut tx, &context, input.owner).await?;
    validate_hole(&mut tx, &context, input.hole_id).await?;
    let actor = score_authorization::authorize_mutation(
        &mut tx,
        input.session_id,
        input.round_id,
        context.scoring_format,
        input.owner,
    )
    .await
    .map_err(mutations::map_authorization_error)?;
    let receipt=sqlx::query_as::<_,Receipt>("SELECT request_hash,applied_score_id,applied_revision FROM score_mutation_receipts WHERE user_id=$1 AND request_id=$2")
        .bind(actor).bind(input.request_id).fetch_optional(&mut *tx).await?;
    if let Some(receipt) = receipt {
        require_live_session(&mut tx, input.session_id).await?;
        if receipt.request_hash != hash {
            return Err(ScorecardError::Conflict(ScorecardConflict::RequestMismatch));
        }
        let revision = ScoreRevision::from_database(receipt.applied_revision)
            .ok_or(ScorecardError::InvalidStoredData)?;
        tx.commit().await?;
        return Ok(MutationResult {
            value: ScoreAcknowledgement {
                request_id: input.request_id,
                applied_score: AppliedScore {
                    score_id: receipt.applied_score_id,
                    revision,
                },
            },
            changed: false,
            tournament_id: context.tournament_id,
        });
    }
    let existing = load_score(&mut tx, input.round_id, input.hole_id, input.owner).await?;
    // Even an equal-value write must first match the reviewed version. This
    // catches absent/present races and A -> B -> A changes without overwriting.
    require_live_session(&mut tx, input.session_id).await?;
    if !input.expected_score.matches(existing.as_ref()) {
        return Err(ScorecardError::Conflict(ScorecardConflict::VersionConflict));
    }
    let result = mutations::save_with_actor(
        &mut tx,
        &context,
        SaveScore {
            round_id: input.round_id,
            hole_id: input.hole_id,
            owner: input.owner,
            gross_strokes: input.gross_strokes,
            submitted_by: actor,
        },
        actor,
    )
    .await?;
    mutations::set_mutation_context(&mut tx, input.round_id).await?;
    sqlx::query("SELECT set_config('app.conditional_score_user_id',$1::text,true),set_config('app.conditional_score_request_id',$2::text,true)")
        .bind(actor).bind(input.request_id).execute(&mut *tx).await?;
    // Different rounds can race with the same account/request ID. The unique
    // key waits for the winner; a loser rolls its entire score transaction back.
    let inserted=sqlx::query("INSERT INTO score_mutation_receipts(user_id,request_id,round_id,request_hash,applied_score_id,applied_revision) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(user_id,request_id) DO NOTHING")
        .bind(actor).bind(input.request_id).bind(input.round_id).bind(hash).bind(result.value.id).bind(result.value.revision.as_i64()).execute(&mut *tx).await?.rows_affected()==1;
    require_live_session(&mut tx, input.session_id).await?;
    if !inserted {
        return Err(ScorecardError::Conflict(ScorecardConflict::RequestMismatch));
    }
    tx.commit().await?;
    Ok(MutationResult {
        value: ScoreAcknowledgement {
            request_id: input.request_id,
            applied_score: AppliedScore {
                score_id: result.value.id,
                revision: result.value.revision,
            },
        },
        changed: result.changed,
        tournament_id: context.tournament_id,
    })
}
async fn require_live_session(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
) -> Result<(), ScorecardError> {
    auth::lock_active_session(tx, session)
        .await?
        .ok_or(ScorecardError::Unauthenticated)?;
    Ok(())
}
