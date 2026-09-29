#![cfg(feature = "database-tests")]
#[path = "tournament_details_support/mod.rs"]
mod support;
use support::*;
#[sqlx::test(migrations = "../migrations")]
async fn edits_only_details_emits_once_and_noop_preserves_version(pool: PgPool) {
    seed(&pool).await;
    let unchanged = preserved(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(Arc::clone(&state));
    let response = app
        .clone()
        .oneshot(request(TOKEN, payload(&pool).await))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let saved = json_body(response).await;
    assert_eq!(saved["name"], "Updated");
    assert_eq!(saved["description"], "New description");
    assert!(events.try_recv().is_ok());
    assert_eq!(preserved(&pool).await, unchanged);
    let response = app
        .oneshot(request(TOKEN, payload(&pool).await))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await, saved);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}
#[sqlx::test(migrations = "../migrations")]
async fn rejects_invalid_shape_dates_bounds_and_all_nonadmin_roles(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    let good = payload(&pool).await;
    for (field, value) in [
        ("name", json!(" ")),
        ("name", json!("æ".repeat(61))),
        ("description", json!("x".repeat(2001))),
        ("start_date", json!("2026-02-30")),
        ("start_date", json!("2026-09-05")),
        ("status", json!("active")),
    ] {
        let mut data = good.clone();
        data[field] = value;
        assert_eq!(
            app.clone()
                .oneshot(request(TOKEN, data))
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let mut narrow = good.clone();
    narrow["start_date"] = json!("2026-09-03");
    let r = app.clone().oneshot(request(TOKEN, narrow)).await.unwrap();
    assert_eq!(r.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_body(r).await["error"]["code"],
        "tournament_details_round_dates"
    );
    for role in ["viewer", "player", "scorer"] {
        sqlx::query("UPDATE tournament_memberships SET role=$1::tournament_role WHERE user_id=$2")
            .bind(role)
            .bind(OTHER)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            app.clone()
                .oneshot(request(OTHER_TOKEN, good.clone()))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    let mut no_csrf = request(TOKEN, good.clone());
    no_csrf.headers_mut().remove("x-csrf-token");
    assert_eq!(
        app.clone().oneshot(no_csrf).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.oneshot(request("missing", good))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn stale_edit_and_start_races_have_one_authoritative_winner(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    let good = payload(&pool).await;
    let (a, b) = tokio::join!(
        app.clone().oneshot(request(TOKEN, good.clone())),
        app.clone().oneshot(request(TOKEN, good.clone()))
    );
    let statuses = [a.unwrap().status(), b.unwrap().status()];
    assert!(statuses.contains(&StatusCode::OK));
    assert!(statuses.contains(&StatusCode::CONFLICT));
    let input = payload(&pool).await;
    let start = Request::post(format!("/api/tournaments/{TRIP}/start"))
        .header(header::COOKIE, format!("golf_session={TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(TOKEN))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"expected_tournament_updated_at":input["expected_tournament_updated_at"]})
                .to_string(),
        ))
        .unwrap();
    let mut edit = input.clone();
    edit["name"] = json!("Race edit");
    let (a, b) = tokio::join!(
        app.clone().oneshot(request(TOKEN, edit)),
        app.clone().oneshot(start)
    );
    let statuses = [a.unwrap().status(), b.unwrap().status()];
    assert!(statuses.contains(&StatusCode::OK));
    assert!(statuses.contains(&StatusCode::CONFLICT));
    // Start, if the edit won first; an active tournament must reject even no-op edits.
    let version = payload(&pool).await;
    let started =
        sqlx::query_scalar::<_, String>("SELECT status::text FROM tournaments WHERE id=$1")
            .bind(TRIP)
            .fetch_one(&pool)
            .await
            .unwrap();
    if started == "draft" {
        let sid = sqlx::query_scalar("SELECT id FROM user_sessions WHERE user_id=$1")
            .bind(ADMIN)
            .fetch_one(&pool)
            .await
            .unwrap();
        let at = serde_json::from_value(version["expected_tournament_updated_at"].clone()).unwrap();
        golf_api::repositories::tournaments::start_authorized(&pool, sid, TRIP, at)
            .await
            .unwrap();
    }
    assert_eq!(
        app.oneshot(request(TOKEN, payload(&pool).await))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn database_rejects_unscoped_details_and_outside_rounds(pool: PgPool) {
    seed(&pool).await;
    assert!(
        sqlx::query("UPDATE tournaments SET name='Bypass' WHERE id=$1")
            .bind(TRIP)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE rounds SET round_date='2026-09-04' WHERE tournament_id=$1")
            .bind(TRIP)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES ($1,$2,2,'Outside','2026-09-04','','','individual_stroke_play')").bind(Uuid::new_v4()).bind(TRIP).execute(&pool).await.is_err());
}
#[sqlx::test(migrations = false)]
async fn schema32_upgrade_preserves_details_and_rounds(pool: PgPool) {
    for migration in golf_api::schema::MIGRATOR
        .iter()
        .filter(|m| m.version <= 32)
    {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    seed(&pool).await;
    let before = preserved(&pool).await;
    sqlx::raw_sql(include_str!("../../migrations/0033_tournament_details.sql"))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(preserved(&pool).await, before);
    assert_eq!(
        api::router(AppState::new(pool.clone()))
            .oneshot(request(TOKEN, payload(&pool).await))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}
