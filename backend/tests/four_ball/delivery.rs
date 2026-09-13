use super::support::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use golf_api::{
    AppState, api,
    auth::derive_csrf_token,
    domain::{
        four_ball_card::Input,
        scorecards::{ExpectedScore, ScoreOwner},
    },
    repositories::{
        four_ball, score_authorization,
        scorecards::{self, ScorecardConflict, ScorecardError},
    },
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
fn present(ack: golf_api::domain::scorecards::ScoreAcknowledgement) -> ExpectedScore {
    ExpectedScore::Present {
        score_id: ack.applied_score.score_id,
        revision: ack.applied_score.revision,
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn numeric_pickup_restore_preserves_identity_detects_aba_and_replays_receipt(pool: PgPool) {
    let session = ready(&pool).await;
    let mut op = operation(
        session,
        11,
        1,
        Input::Numeric { gross_strokes: 4 },
        ExpectedScore::Absent {},
    );
    let original_id = op.request_id;
    let first = four_ball::save_conditional(&pool, op).await.unwrap().value;
    op = operation(session, 11, 1, Input::NoScore {}, present(first));
    let second = four_ball::save_conditional(&pool, op).await.unwrap().value;
    assert_eq!(first.applied_score.score_id, second.applied_score.score_id);
    assert_eq!(second.applied_score.revision.as_i64(), 2);
    let third = four_ball::save_conditional(
        &pool,
        operation(
            session,
            11,
            1,
            Input::Numeric { gross_strokes: 4 },
            present(second),
        ),
    )
    .await
    .unwrap()
    .value;
    assert_eq!(third.applied_score.score_id, first.applied_score.score_id);
    assert_eq!(third.applied_score.revision.as_i64(), 3);
    assert!(matches!(
        four_ball::save_conditional(
            &pool,
            operation(
                session,
                11,
                1,
                Input::Numeric { gross_strokes: 4 },
                present(first)
            )
        )
        .await,
        Err(ScorecardError::Conflict(ScorecardConflict::VersionConflict))
    ));
    let mut retry = operation(
        session,
        11,
        1,
        Input::Numeric { gross_strokes: 4 },
        ExpectedScore::Absent {},
    );
    retry.request_id = original_id;
    let replay = four_ball::save_conditional(&pool, retry).await.unwrap();
    assert!(!replay.changed);
    assert_eq!(replay.value, first);
    let audit: Vec<(i64, Option<i16>)> = sqlx::query_as(
        "SELECT revision,gross_strokes FROM four_ball_input_audits ORDER BY revision",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(audit, vec![(1, Some(4)), (2, None), (3, Some(4))]);
}
#[sqlx::test(migrations = "../migrations")]
async fn side_selection_and_player_authority_reject_other_flight_viewer_and_legacy_paths(
    pool: PgPool,
) {
    let admin = ready(&pool).await;
    let player = session(&pool, 41, Some(11), "player", "fb-player").await;
    let viewer = session(&pool, 42, None, "viewer", "fb-viewer").await;
    assert_eq!(
        score_authorization::writable_owners(&pool, player, id(5))
            .await
            .unwrap(),
        vec![ScoreOwner::Team { id: id(21) }]
    );
    numeric(&pool, player, 12, 1, 5).await;
    for sess in [player, viewer] {
        assert!(matches!(
            four_ball::save_conditional(
                &pool,
                operation(sess, 13, 1, Input::NoScore {}, ExpectedScore::Absent {})
            )
            .await,
            Err(ScorecardError::Forbidden)
        ));
    }
    for owner in [
        ScoreOwner::Team { id: id(21) },
        ScoreOwner::Player { id: id(11) },
    ] {
        assert!(matches!(
            scorecards::get_read_authenticated(&pool, admin, id(5), owner).await,
            Err(ScorecardError::Conflict(
                ScorecardConflict::OwnerFormatMismatch
            ))
        ));
        assert!(matches!(
            scorecards::save_authenticated(
                &pool,
                scorecards::AuthenticatedSaveScore {
                    round_id: id(5),
                    hole_id: id(101),
                    owner,
                    gross_strokes: 4,
                    session_id: admin
                }
            )
            .await,
            Err(ScorecardError::Conflict(
                ScorecardConflict::OwnerFormatMismatch
            ))
        ));
    }
    assert!(
        four_ball::get(&pool, viewer, id(5), id(21), true)
            .await
            .is_err()
    );
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
        .bind(id(2))
        .bind(id(42))
        .execute(&pool)
        .await
        .unwrap();
    for scoring in [false, true] {
        assert!(matches!(
            four_ball::get(&pool, viewer, id(5), id(21), scoring).await,
            Err(ScorecardError::Forbidden)
        ));
    }
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true)")
        .bind(id(5))
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(sqlx::query("INSERT INTO scores(id,round_id,tournament_id,hole_id,player_id,gross_strokes,submitted_by) VALUES($1,$2,$3,$4,$5,4,$6)").bind(Uuid::new_v4()).bind(id(5)).bind(id(2)).bind(id(101)).bind(id(11)).bind(id(1)).execute(&mut *tx).await.is_err());
}
async fn request(app: &axum::Router, path: String, token: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::get(path)
                .header("cookie", format!("golf_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
#[sqlx::test(migrations = "../migrations")]
async fn private_read_is_actor_free_and_scoring_retains_revision_and_strict_api(pool: PgPool) {
    let session = ready(&pool).await;
    numeric(&pool, session, 11, 1, 4).await;
    let app = api::router(AppState::new(pool));
    let path = format!("/api/rounds/{}/four-ball/scorecards/{}", id(5), id(21));
    let read = request(&app, path.clone(), TOKEN).await;
    assert_eq!(
        read["holes"][0]["players"][0]["score"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    assert!(read.get("confirmed_by").is_none());
    let scoring = request(&app, format!("{path}/scoring"), TOKEN).await;
    assert_eq!(scoring["holes"][0]["players"][0]["score"]["revision"], "1");
    for input in [
        json!({"type":"numeric","gross_strokes":0}),
        json!({"type":"no_score","gross_strokes":4}),
    ] {
        let body = json!({"request_id":Uuid::new_v4(),"hole_id":id(101),"owner":{"type":"player","id":id(11)},"input":input,"expected_score":{"type":"absent"}});
        let response = app
            .clone()
            .oneshot(
                Request::put(format!(
                    "/api/rounds/{}/four-ball/inputs/conditional",
                    id(5)
                ))
                .header("cookie", format!("golf_session={TOKEN}"))
                .header("x-csrf-token", derive_csrf_token(TOKEN))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn sql_boundary_rejects_bypass_identity_revision_and_history_mutations(pool: PgPool) {
    let session = ready(&pool).await;
    numeric(&pool, session, 11, 1, 4).await;
    for query in [
        "UPDATE four_ball_inputs SET gross_strokes=5",
        "DELETE FROM four_ball_inputs",
        "UPDATE four_ball_input_audits SET gross_strokes=5",
        "DELETE FROM four_ball_input_audits",
        "UPDATE four_ball_mutation_receipts SET applied_revision=2",
        "DELETE FROM four_ball_mutation_receipts",
    ] {
        assert!(sqlx::query(query).execute(&pool).await.is_err(), "{query}");
    }
    for query in [
        "UPDATE four_ball_inputs SET revision=2",
        "UPDATE four_ball_inputs SET id=gen_random_uuid()",
    ] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true),set_config('app.four_ball_actor_id',$2::text,true),set_config('app.four_ball_session_id',$3::text,true)").bind(id(5)).bind(id(1)).bind(session).execute(&mut *tx).await.unwrap();
        assert!(sqlx::query(query).execute(&mut *tx).await.is_err());
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT revision FROM four_ball_inputs")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn ancestor_deletion_cascades_inputs_audits_and_receipts(pool: PgPool) {
    let session = ready(&pool).await;
    numeric(&pool, session, 11, 1, 4).await;
    sqlx::query("DELETE FROM tournaments WHERE id=$1")
        .bind(id(2))
        .execute(&pool)
        .await
        .unwrap();
    for query in [
        "SELECT count(*) FROM four_ball_inputs",
        "SELECT count(*) FROM four_ball_input_audits",
        "SELECT count(*) FROM four_ball_mutation_receipts",
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(query)
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn private_get_checks_membership_before_format_mismatch(pool: PgPool) {
    draft(&pool, "team_scramble").await;
    golf_api::repositories::round_lifecycle::open(&pool, id(5))
        .await
        .unwrap();
    session(&pool, 41, None, "viewer", "outsider").await;
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
        .bind(id(2))
        .bind(id(41))
        .execute(&pool)
        .await
        .unwrap();
    let app = api::router(AppState::new(pool));
    for (round, token, expected) in [
        (id(5), "outsider", StatusCode::FORBIDDEN),
        (id(5), TOKEN, StatusCode::CONFLICT),
        (Uuid::new_v4(), "outsider", StatusCode::NOT_FOUND),
    ] {
        for suffix in ["", "/scoring"] {
            let response = app
                .clone()
                .oneshot(
                    Request::get(format!(
                        "/api/rounds/{round}/four-ball/scorecards/{}{suffix}",
                        id(21)
                    ))
                    .header("cookie", format!("golf_session={token}"))
                    .body(Body::empty())
                    .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
            assert_eq!(response.headers()["cache-control"], "private, no-store");
        }
    }
}
