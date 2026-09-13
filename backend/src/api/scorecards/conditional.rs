use super::*;
use crate::domain::scorecards::{ExpectedScore, ScoreAcknowledgement};
use crate::repositories::scorecards::ConditionalSaveScore;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    request_id: Uuid,
    hole_id: Uuid,
    owner: Owner,
    gross_strokes: i16,
    expected_score: ExpectedScore,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Owner {
    Player { id: Uuid },
    Team { id: Uuid },
}
impl From<Owner> for ScoreOwner {
    fn from(owner: Owner) -> Self {
        match owner {
            Owner::Player { id } => Self::Player { id },
            Owner::Team { id } => Self::Team { id },
        }
    }
}
pub(super) async fn save(
    State(state): State<Arc<AppState>>,
    MutationSession(authenticated): MutationSession,
    Path(round): Path<String>,
    input: Result<Json<Request>, JsonRejection>,
) -> ApiResult<Json<ScoreAcknowledgement>> {
    let round_id = round
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid round identifier".into()))?;
    let Json(input) =
        input.map_err(|_| ApiError::BadRequest("invalid conditional score request".into()))?;
    if !(1..=20).contains(&input.gross_strokes) {
        return Err(ApiError::BadRequest(
            "gross_strokes must be between 1 and 20".into(),
        ));
    }
    let result = scorecards::save_conditional(
        &state.pool,
        ConditionalSaveScore {
            request_id: input.request_id,
            round_id,
            hole_id: input.hole_id,
            owner: input.owner.into(),
            gross_strokes: input.gross_strokes,
            expected_score: input.expected_score,
            session_id: authenticated.principal.session_id,
        },
    )
    .await
    .map_err(|error| match error {
        // Receipt/database constraint details can contain score payload metadata.
        ScorecardError::Database(_) => ApiError::Internal,
        other => map_error(other),
    })?;
    if result.changed {
        state.notify("score", result.tournament_id, round_id);
    }
    Ok(Json(result.value))
}
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/rounds/{round_id}/scores/conditional", put(save))
        .layer(axum::extract::DefaultBodyLimit::max(2048))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("private, no-store"),
                );
                response
            },
        ))
}
