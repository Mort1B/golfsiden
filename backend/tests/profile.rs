#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_password, hash_session_token},
    repositories::{
        auth,
        profile::{self, CredentialChange, DetailsChange, ProfileError},
    },
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};
const USER: Uuid = uuid!("00000000-0000-0000-0000-000000001001");
const ADMIN: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
const TRIP: Uuid = uuid!("00000000-0000-0000-0000-000000002001");
const TOKEN: &str = "profile-test-session";
const PASSWORD: &str = "profile-old-password";
async fn fixture(pool: &PgPool) -> Uuid {
    sqlx::raw_sql(include_str!("../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET password_hash=$2 WHERE id=$1")
        .bind(USER)
        .bind(hash_password(PASSWORD.as_bytes()).unwrap())
        .execute(pool)
        .await
        .unwrap();
    session(pool, USER, TOKEN).await
}
async fn session(pool: &PgPool, user: Uuid, token: &str) -> Uuid {
    auth::create_session(
        pool,
        user,
        &hash_session_token(token),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id
}
fn request(
    method: &str,
    path: &str,
    value: Value,
    token: Option<&str>,
    csrf: bool,
) -> Request<Body> {
    let mut r = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        r = r.header("cookie", format!("golf_session={token}"));
        if csrf {
            r = r.header("x-csrf-token", derive_csrf_token(token));
        }
    }
    r.body(Body::from(value.to_string())).unwrap()
}
async fn body(r: axum::response::Response) -> Value {
    serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
async fn details(pool: &PgPool, sid: Uuid) -> Value {
    let p = profile::get(pool, sid).await.unwrap();
    json!({"version":p.version,"player_updated_at":p.player_updated_at,"display_name":p.display_name,"handicap":p.handicap,"reason":""})
}

#[sqlx::test(migrations = "../migrations")]
async fn profile_is_self_only_private_csrf_and_strictly_validated(pool: PgPool) {
    let sid = fixture(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    for path in [
        "/api/me/profile",
        "/api/me/profile/username",
        "/api/me/profile/password",
    ] {
        let method = if path == "/api/me/profile" {
            "PUT"
        } else {
            "POST"
        };
        for (token, csrf, status) in [
            (None, false, StatusCode::UNAUTHORIZED),
            (Some(TOKEN), false, StatusCode::FORBIDDEN),
        ] {
            let r = app
                .clone()
                .oneshot(request(method, path, json!({}), token, csrf))
                .await
                .unwrap();
            assert_eq!(r.status(), status);
            assert_eq!(r.headers()["cache-control"], "private, no-store");
        }
    }
    let r = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/me/profile",
            json!({}),
            Some(TOKEN),
            false,
        ))
        .await
        .unwrap();
    assert_eq!(r.headers()["cache-control"], "private, no-store");
    let p = body(r).await;
    assert_eq!(p["user_id"], USER.to_string());
    assert!(p.get("password_hash").is_none());
    let original = details(&pool, sid).await;
    for (field, value) in [
        ("user_id", json!(ADMIN)),
        ("role", json!("admin")),
        ("handicap", json!(54.1)),
        ("handicap", json!(1.23)),
        ("display_name", json!(" ")),
        ("reason", json!("a".repeat(501))),
    ] {
        let mut invalid = original.clone();
        invalid[field] = value;
        assert_eq!(
            app.clone()
                .oneshot(request(
                    "PUT",
                    "/api/me/profile",
                    invalid,
                    Some(TOKEN),
                    true
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(details(&pool, sid).await, original);
}

#[sqlx::test(migrations = "../migrations")]
async fn details_preserve_tournament_history_append_handicap_audit_and_reject_stale(pool: PgPool) {
    let sid = fixture(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let before:Value=sqlx::query_scalar("SELECT jsonb_build_object('entrants',(SELECT jsonb_agg(to_jsonb(t)) FROM tournament_players t),'rounds',(SELECT jsonb_agg(to_jsonb(r)) FROM round_handicap_snapshots r),'teams',(SELECT jsonb_agg(to_jsonb(t)) FROM round_team_handicap_snapshots t),'scores',(SELECT jsonb_agg(to_jsonb(s)) FROM scores s))").fetch_one(&pool).await.unwrap();
    let mut input = details(&pool, sid).await;
    input["display_name"] = json!("Updated profile name");
    input["handicap"] = json!(14.4);
    input["reason"] = json!("New official handicap");
    let result = app
        .clone()
        .oneshot(request(
            "PUT",
            "/api/me/profile",
            input.clone(),
            Some(TOKEN),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(result.status(), StatusCode::OK);
    let p = body(result).await;
    assert_eq!(p["handicap"], 14.4);
    assert_eq!(p["display_name"], "Updated profile name");
    assert_eq!(events.try_recv().unwrap().resource, "tournament");
    let audit: (String, Uuid) = sqlx::query_as(
        "SELECT reason,changed_by FROM handicap_history WHERE player_id=$1 AND handicap_index=14.4",
    )
    .bind(USER)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit, ("New official handicap".into(), USER));
    let after:Value=sqlx::query_scalar("SELECT jsonb_build_object('entrants',(SELECT jsonb_agg(to_jsonb(t)) FROM tournament_players t),'rounds',(SELECT jsonb_agg(to_jsonb(r)) FROM round_handicap_snapshots r),'teams',(SELECT jsonb_agg(to_jsonb(t)) FROM round_team_handicap_snapshots t),'scores',(SELECT jsonb_agg(to_jsonb(s)) FROM scores s))").fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
    let stale = app
        .oneshot(request("PUT", "/api/me/profile", input, Some(TOKEN), true))
        .await
        .unwrap();
    assert_eq!(body(stale).await["error"]["code"], "profile_stale");
    assert_eq!(
        auth::find_active_session(&pool, &hash_session_token(TOKEN))
            .await
            .unwrap()
            .unwrap()
            .display_name,
        "Updated profile name"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn unlinked_self_profile_never_enrolls_old_trips_and_inactive_handicap_fails(pool: PgPool) {
    let sid = fixture(&pool).await;
    let admin = session(&pool, ADMIN, "profile-admin").await;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM tournament_players")
        .fetch_one(&pool)
        .await
        .unwrap();
    let old = profile::get(&pool, admin).await.unwrap();
    assert!(old.player_id.is_none());
    let result = profile::update_details(
        &pool,
        admin,
        DetailsChange {
            version: old.version,
            player_updated_at: None,
            display_name: old.display_name,
            handicap: Some(9.1),
            reason: "Created my player profile".into(),
        },
    )
    .await
    .unwrap();
    assert!(result.profile.player_id.is_some());
    assert_ne!(result.profile.player_id, Some(USER));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_players")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before
    );
    sqlx::query("UPDATE players SET active=false WHERE id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    let p = profile::get(&pool, sid).await.unwrap();
    let err = profile::update_details(
        &pool,
        sid,
        DetailsChange {
            version: p.version,
            player_updated_at: p.player_updated_at,
            display_name: p.display_name,
            handicap: Some(9.1),
            reason: "Self edit".into(),
        },
    )
    .await
    .err()
    .unwrap();
    assert!(matches!(err, ProfileError::Inactive));
}

#[sqlx::test(migrations = "../migrations")]
async fn username_reauth_duplicate_normalization_and_old_login_are_deliberate(pool: PgPool) {
    let sid = fixture(&pool).await;
    let p = profile::get(&pool, sid).await.unwrap();
    let verified = profile::credential(&pool, USER).await.unwrap().unwrap();
    let app = api::router(AppState::new(pool.clone()));
    for (username, password, code) in [
        (
            "new-profile-name",
            "wrong-password",
            "current_password_incorrect",
        ),
        ("ADMIN", PASSWORD, "username_unavailable"),
    ] {
        let r = app
            .clone()
            .oneshot(request(
                "POST",
                "/api/me/profile/username",
                json!({"version":p.version,"username":username,"current_password":password}),
                Some(TOKEN),
                true,
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::CONFLICT);
        assert_eq!(body(r).await["error"]["code"], code);
    }
    let r=app.clone().oneshot(request("POST","/api/me/profile/username",json!({"version":p.version,"username":" New-Profile_Name ","current_password":PASSWORD}),Some(TOKEN),true)).await.unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        profile::get(&pool, sid).await.unwrap().username,
        "new-profile_name"
    );
    assert!(
        auth::create_verified_session(
            &pool,
            USER,
            &hash_session_token("stale-login"),
            Utc::now() + Duration::hours(1),
            (&verified.0, &p.username, verified.1)
        )
        .await
        .unwrap()
        .is_none()
    );
    let login = app
        .oneshot(request(
            "POST",
            "/api/auth/login",
            json!({"username":"new-profile_name","password":PASSWORD}),
            None,
            false,
        ))
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../migrations")]
async fn password_revokes_all_devices_and_all_auth_locks_then_allows_new_login(pool: PgPool) {
    let sid = fixture(&pool).await;
    let second = session(&pool, USER, "profile-second").await;
    let p = profile::get(&pool, sid).await.unwrap();
    let verified = profile::credential(&pool, USER).await.unwrap().unwrap();
    let app = api::router(AppState::new(pool.clone()));
    let r=app.clone().oneshot(request("POST","/api/me/profile/password",json!({"version":p.version,"new_password":"profile-new-password","current_password":PASSWORD}),Some(TOKEN),true)).await.unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert!(
        r.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    for (token, sid) in [(TOKEN, sid), ("profile-second", second)] {
        assert!(
            auth::find_active_session(&pool, &hash_session_token(token))
                .await
                .unwrap()
                .is_none()
        );
        let mut tx = pool.begin().await.unwrap();
        assert!(
            auth::lock_active_session(&mut tx, sid)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            auth::lock_active_session_exclusive(&mut tx, sid)
                .await
                .unwrap()
                .is_none()
        );
    }
    assert!(
        auth::create_verified_session(
            &pool,
            USER,
            &hash_session_token("old-login"),
            Utc::now() + Duration::hours(1),
            (&verified.0, &p.username, verified.1)
        )
        .await
        .unwrap()
        .is_none()
    );
    for (password, status) in [
        (PASSWORD, StatusCode::UNAUTHORIZED),
        ("profile-new-password", StatusCode::OK),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(
                    "POST",
                    "/api/auth/login",
                    json!({"username":p.username,"password":password}),
                    None,
                    false
                ))
                .await
                .unwrap()
                .status(),
            status
        );
    }
    assert!(matches!(
        profile::update_credential(
            &pool,
            second,
            p.version,
            &verified,
            CredentialChange::Username("cannot-change".into())
        )
        .await,
        Err(ProfileError::Unauthenticated)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn profile_credentials_are_throttled_and_oversized_bodies_rejected(pool: PgPool) {
    let sid = fixture(&pool).await;
    let p = profile::get(&pool, sid).await.unwrap();
    let state = AppState::with_runtime_services(
        pool.clone(),
        golf_api::auth::AuthConfig::local(),
        golf_api::course_provider::CourseProviderClient::disabled(),
        golf_api::rate_limit::RateLimiter::with_rules(
            [(
                golf_api::rate_limit::RateLimitRoute::ProfileCredentials,
                std::time::Duration::from_secs(60),
                1,
                2,
            )],
            16,
        ),
    );
    let app = api::router(state);
    let value =
        json!({"version":p.version,"username":"a-new-name","current_password":"wrong-password"});
    assert_eq!(
        app.clone()
            .oneshot(request(
                "POST",
                "/api/me/profile/username",
                value.clone(),
                Some(TOKEN),
                true
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let r = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/me/profile/username",
            value,
            Some(TOKEN),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(r.headers().contains_key("retry-after"));
    assert_eq!(
        app.oneshot(request(
            "PUT",
            "/api/me/profile",
            json!({"display_name":"a".repeat(9000)}),
            Some(TOKEN),
            true
        ))
        .await
        .unwrap()
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn old_generation_cannot_authorize_direct_visibility_guard(pool: PgPool) {
    fixture(&pool).await;
    let sid = session(&pool, ADMIN, "old-admin").await;
    sqlx::query("UPDATE users SET password_hash='replacement' WHERE id=$1")
        .bind(ADMIN)
        .execute(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.final_round_visibility_tournament_id',$1,true),set_config('app.final_round_visibility_session_id',$2,true)").bind(TRIP.to_string()).bind(sid.to_string()).execute(&mut *tx).await.unwrap();
    let err = sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden=false WHERE id=$1")
        .bind(TRIP)
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(
        err.as_database_error().unwrap().constraint(),
        Some("final_round_visibility_admin_required")
    );
}
