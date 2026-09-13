use super::*;
use crate::domain::{
    four_ball_card::Input,
    scorecards::{AppliedScore, ExpectedScore, ScoreAcknowledgement, ScoreRevision},
};
use crate::repositories::scorecards::MutationResult;
use sha2::{Digest, Sha256};
#[derive(serde::Serialize)]
pub struct SaveInput {
    pub request_id: Uuid,
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub owner: ScoreOwner,
    pub input: Input,
    pub expected_score: ExpectedScore,
    #[serde(skip)]
    pub session_id: Uuid,
}
#[derive(sqlx::FromRow)]
struct Receipt {
    request_hash: Vec<u8>,
    applied_score_id: Uuid,
    applied_revision: i64,
}

pub async fn save_conditional(
    pool: &PgPool,
    input: SaveInput,
) -> Result<MutationResult<ScoreAcknowledgement>, ScorecardError> {
    input
        .input
        .domain()
        .map_err(|_| ScorecardError::InvalidStoredData)?;
    let hash =
        Sha256::digest(serde_json::to_vec(&input).map_err(|_| ScorecardError::InvalidStoredData)?)
            .to_vec();
    let mut tx = pool.begin().await?;
    let context = context(&mut tx, input.round_id, true).await?;
    editable(&context)?;
    let player = input.owner.player_id().ok_or(ScorecardError::Conflict(
        ScorecardConflict::OwnerFormatMismatch,
    ))?;
    let team = sqlx::query_scalar::<_, Uuid>(
        "SELECT team_id FROM team_memberships WHERE round_id=$1 AND player_id=$2",
    )
    .bind(input.round_id)
    .bind(player)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ScorecardError::Conflict(
        ScorecardConflict::OwnerNotEligible,
    ))?;
    partners(&mut tx, input.round_id, team).await?;
    let valid_hole = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM holes WHERE id=$1 AND tee_id=$2)",
    )
    .bind(input.hole_id)
    .bind(context.tee_id)
    .fetch_one(&mut *tx)
    .await?;
    if !valid_hole {
        return Err(ScorecardError::Conflict(ScorecardConflict::HoleMismatch));
    }
    let actor = score_authorization::authorize_mutation(
        &mut tx,
        input.session_id,
        input.round_id,
        context.scoring_format,
        input.owner,
    )
    .await
    .map_err(map_authorization)?;
    let receipt=sqlx::query_as::<_,Receipt>("SELECT request_hash,applied_score_id,applied_revision FROM four_ball_mutation_receipts WHERE user_id=$1 AND request_id=$2").bind(actor).bind(input.request_id).fetch_optional(&mut *tx).await?;
    if let Some(receipt) = receipt {
        live(&mut tx, input.session_id).await?;
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
    let current=sqlx::query_as::<_,InputRow>("SELECT id,revision,round_id,hole_id,player_id,gross_strokes,submitted_by,submitted_at,updated_at FROM four_ball_inputs WHERE round_id=$1 AND hole_id=$2 AND player_id=$3 FOR UPDATE").bind(input.round_id).bind(input.hole_id).bind(player).fetch_optional(&mut *tx).await?;
    let matches = match (input.expected_score, current.as_ref()) {
        (ExpectedScore::Absent {}, None) => true,
        (ExpectedScore::Present { score_id, revision }, Some(current)) => {
            score_id == current.id && revision.as_i64() == current.revision
        }
        _ => false,
    };
    live(&mut tx, input.session_id).await?;
    if !matches {
        return Err(ScorecardError::Conflict(ScorecardConflict::VersionConflict));
    }
    let changed = current
        .as_ref()
        .is_none_or(|row| row.gross_strokes != input.input.gross());
    set_context(&mut tx, input.round_id, input.session_id, actor).await?;
    let (id, revision) = if changed {
        sqlx::query_as::<_,(Uuid,i64)>("INSERT INTO four_ball_inputs(id,round_id,tournament_id,hole_id,player_id,gross_strokes,submitted_by) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(round_id,hole_id,player_id) DO UPDATE SET gross_strokes=EXCLUDED.gross_strokes,submitted_by=EXCLUDED.submitted_by RETURNING id,revision")
            .bind(Uuid::new_v4()).bind(input.round_id).bind(context.tournament_id).bind(input.hole_id).bind(player).bind(input.input.gross()).bind(actor).fetch_one(&mut *tx).await?
    } else {
        let row = current.ok_or(ScorecardError::InvalidStoredData)?;
        (row.id, row.revision)
    };
    sqlx::query("SELECT set_config('app.four_ball_request_id',$1::text,true)")
        .bind(input.request_id)
        .execute(&mut *tx)
        .await?;
    let inserted=sqlx::query("INSERT INTO four_ball_mutation_receipts(user_id,request_id,round_id,request_hash,applied_score_id,applied_revision) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(user_id,request_id) DO NOTHING").bind(actor).bind(input.request_id).bind(input.round_id).bind(hash).bind(id).bind(revision).execute(&mut *tx).await?.rows_affected()==1;
    if !inserted {
        return Err(ScorecardError::Conflict(ScorecardConflict::RequestMismatch));
    }
    live(&mut tx, input.session_id).await?;
    tx.commit().await?;
    Ok(MutationResult {
        value: ScoreAcknowledgement {
            request_id: input.request_id,
            applied_score: AppliedScore {
                score_id: id,
                revision: ScoreRevision::from_database(revision)
                    .ok_or(ScorecardError::InvalidStoredData)?,
            },
        },
        changed,
        tournament_id: context.tournament_id,
    })
}
pub async fn confirm(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    team_id: Uuid,
) -> Result<MutationResult<Card>, ScorecardError> {
    let mut tx = pool.begin().await?;
    let context = context(&mut tx, round_id, true).await?;
    editable(&context)?;
    let (name, partners) = partners(&mut tx, round_id, team_id).await?;
    let actor = authorize_partners(&mut tx, session_id, &context, &partners).await?;
    let mut card = build(&mut tx, &context, team_id, name, partners, unrestricted()).await?;
    if card.complete != Some(true) {
        return Err(ScorecardError::Conflict(ScorecardConflict::Incomplete));
    }
    let changed = card.confirmed != Some(true);
    if changed {
        set_context(&mut tx, round_id, session_id, actor).await?;
        let confirmed_at=sqlx::query_scalar("INSERT INTO scorecard_confirmations(id,round_id,tournament_id,team_id,confirmed_by) VALUES($1,$2,$3,$4,$5) RETURNING confirmed_at").bind(Uuid::new_v4()).bind(round_id).bind(context.tournament_id).bind(team_id).bind(actor).fetch_one(&mut *tx).await?;
        card.confirmed = Some(true);
        card.confirmed_at = Some(confirmed_at);
    }
    live(&mut tx, session_id).await?;
    tx.commit().await?;
    Ok(MutationResult {
        value: card,
        changed,
        tournament_id: context.tournament_id,
    })
}
async fn live(tx: &mut Transaction<'_, Postgres>, session: Uuid) -> Result<(), ScorecardError> {
    auth::lock_active_session(tx, session)
        .await?
        .ok_or(ScorecardError::Unauthenticated)?;
    Ok(())
}
async fn set_context(
    tx: &mut Transaction<'_, Postgres>,
    round: Uuid,
    session: Uuid,
    actor: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true),set_config('app.four_ball_session_id',$2::text,true),set_config('app.four_ball_actor_id',$3::text,true)").bind(round).bind(session).bind(actor).execute(&mut **tx).await?;
    Ok(())
}
