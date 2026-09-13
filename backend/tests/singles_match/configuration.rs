use super::*;
use axum::{
    body::Body,
    http::{Request as HttpRequest, StatusCode},
};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    domain::models::{ScoringFormat, ScoringMode},
    domain::tournament_plan::{RoundInput, TournamentPlanInput, normalize},
    repositories::{auth, tournament_plan, tournaments},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
pub(super) async fn admin(pool: &PgPool) -> Uuid {
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES($1,'match_setup_admin','Admin','admin')").bind(id(1)).execute(pool).await.unwrap();
    auth::create_session(
        pool,
        id(1),
        &hash_session_token(TOKEN),
        chrono::Utc::now() + chrono::Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id
}
pub(super) async fn plan(
    pool: &PgPool,
    formats: Vec<ScoringFormat>,
    count: Option<i16>,
) -> (
    golf_api::domain::models::Tournament,
    Vec<golf_api::domain::models::Round>,
) {
    let date = (chrono::Utc::now() + chrono::Duration::days(2)).date_naive();
    let plan = normalize(TournamentPlanInput {
        tournament_name: "Match configuration".into(),
        description: "".into(),
        start_date: date,
        end_date: date,
        counted_rounds: count,
        mandatory_round_number: None,
        rounds: formats
            .into_iter()
            .enumerate()
            .map(|(i, scoring_format)| RoundInput {
                round_number: (i + 1) as i16,
                name: format!("Round {i}"),
                round_date: date,
                scoring_format,
            })
            .collect(),
    })
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let result = tournament_plan::insert(&mut tx, id(1), None, &plan)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    result
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
fn post(path: &str, value: Value, authenticated: bool) -> HttpRequest<Body> {
    let mut r = HttpRequest::post(path).header("content-type", "application/json");
    if authenticated {
        r = r
            .header("cookie", format!("golf_session={TOKEN}"))
            .header("x-csrf-token", derive_csrf_token(TOKEN));
    }
    r.body(Body::from(value.to_string())).unwrap()
}
#[sqlx::test(migrations = "../migrations")]
async fn creation_and_onboarding_require_explicit_nullable_overall_configuration(pool: PgPool) {
    admin(&pool).await;
    let date = (chrono::Utc::now() + chrono::Duration::days(2)).date_naive();
    let app = api::router(AppState::new(pool.clone()));
    let base = json!({"request_id":Uuid::new_v4(),"tournament":{"name":"Singles cup","description":"","start_date":date,"end_date":date,"counted_rounds":null,"mandatory_round_number":null},"rounds":[{"round_number":1,"name":"Singles","round_date":date,"scoring_format":"singles_match_play"}]});
    let mut missing = base.clone();
    missing["tournament"]
        .as_object_mut()
        .unwrap()
        .remove("counted_rounds");
    assert_eq!(
        app.clone()
            .oneshot(post("/api/tournaments", missing, true))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut numeric = base.clone();
    numeric["tournament"]["counted_rounds"] = json!(1);
    assert_eq!(
        app.clone()
            .oneshot(post("/api/tournaments", numeric, true))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let created = app
        .clone()
        .oneshot(post("/api/tournaments", base, true))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let id: Uuid = serde_json::from_value(body(created).await["tournament_id"].clone()).unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, Option<i16>>("SELECT counted_rounds FROM tournaments WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        None
    );
    let onboarding = json!({"creator":{"account":{"username":"match_creator","password":"a secure test password"},"player":{"display_name":"Creator","handicap_index":12.3}},"tournament":{"name":"First singles cup","description":"","start_date":date,"end_date":date,"counted_rounds":null,"mandatory_round_number":null},"rounds":[{"round_number":1,"name":"Singles","round_date":date,"scoring_format":"singles_match_play"}]});
    let mut missing = onboarding.clone();
    missing["tournament"]
        .as_object_mut()
        .unwrap()
        .remove("counted_rounds");
    assert_eq!(
        app.clone()
            .oneshot(post("/api/onboarding/tournaments", missing, false))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let created = app
        .oneshot(post("/api/onboarding/tournaments", onboarding, false))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
}
#[sqlx::test(migrations = "../migrations")]
async fn match_only_noop_and_mixed_count_mandatory_and_start_are_consistent(pool: PgPool) {
    let session = admin(&pool).await;
    let (t, rounds) = plan(&pool, vec![ScoringFormat::SinglesMatchPlay], None).await;
    assert_eq!(t.scoring_mode, ScoringMode::Individual);
    let noop = tournaments::update_counted_rounds_authorized(
        &pool,
        session,
        t.id,
        None,
        None,
        None,
        t.updated_at,
    )
    .await
    .unwrap();
    assert!(!noop.changed);
    assert_eq!(noop.tournament.updated_at, t.updated_at);
    assert!(
        tournaments::update_counted_rounds_authorized(
            &pool,
            session,
            t.id,
            Some(1),
            None,
            None,
            t.updated_at
        )
        .await
        .is_err()
    );
    assert!(
        tournaments::update_counted_rounds_authorized(
            &pool,
            session,
            t.id,
            None,
            Some(rounds[0].id),
            None,
            t.updated_at
        )
        .await
        .is_err()
    );
    sqlx::query(
        "INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,'Entrant',0)",
    )
    .bind(id(11))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES($1,$2,0)").bind(t.id).bind(id(11)).execute(&pool).await.unwrap();
    let updated = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(t.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let started = tournaments::start_authorized(&pool, session, t.id, updated)
        .await
        .unwrap();
    assert!(started.tournament.counted_rounds.is_none());
    let (t, rounds) = plan(
        &pool,
        vec![
            ScoringFormat::IndividualStrokePlay,
            ScoringFormat::SinglesMatchPlay,
        ],
        Some(1),
    )
    .await;
    assert!(
        tournaments::update_counted_rounds_authorized(
            &pool,
            session,
            t.id,
            Some(2),
            None,
            None,
            t.updated_at
        )
        .await
        .is_err()
    );
    assert!(
        tournaments::update_counted_rounds_authorized(
            &pool,
            session,
            t.id,
            Some(1),
            Some(rounds[1].id),
            None,
            t.updated_at
        )
        .await
        .is_err()
    );
    assert!(
        tournaments::update_counted_rounds_authorized(
            &pool,
            session,
            t.id,
            Some(1),
            Some(rounds[0].id),
            None,
            t.updated_at
        )
        .await
        .is_ok()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn deferred_format_guards_serialize_competing_round_changes(pool: PgPool) {
    admin(&pool).await;
    for isolation in ["READ COMMITTED", "REPEATABLE READ"] {
        let (t, rounds) = plan(
            &pool,
            vec![
                ScoringFormat::IndividualStrokePlay,
                ScoringFormat::IndividualStrokePlay,
            ],
            Some(1),
        )
        .await;
        let mut a = pool.begin().await.unwrap();
        let mut b = pool.begin().await.unwrap();
        let statement = format!("SET TRANSACTION ISOLATION LEVEL {isolation}");
        sqlx::query(&statement).execute(&mut *a).await.unwrap();
        sqlx::query(&statement).execute(&mut *b).await.unwrap();
        sqlx::query("UPDATE rounds SET scoring_format='singles_match_play' WHERE id=$1")
            .bind(rounds[0].id)
            .execute(&mut *a)
            .await
            .unwrap();
        sqlx::query("UPDATE rounds SET scoring_format='singles_match_play' WHERE id=$1")
            .bind(rounds[1].id)
            .execute(&mut *b)
            .await
            .unwrap();
        let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            tokio::join!(a.commit(), b.commit())
        })
        .await
        .unwrap();
        assert_ne!(a.is_ok(), b.is_ok(), "{isolation}");
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM rounds WHERE tournament_id=$1 AND scoring_format='singles_match_play'").bind(t.id).fetch_one(&pool).await.unwrap(),1);
        let updated: chrono::DateTime<chrono::Utc> =
            sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
                .bind(t.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            updated, t.updated_at,
            "internal guard must not alter tournament timestamps"
        );
    }
}
