#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header, request::Builder},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{
        derive_csrf_token, generate_invitation_token, hash_invitation_token, hash_session_token,
    },
    repositories::auth,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT_ID: Uuid = uuid!("93000000-0000-0000-0000-000000000001");
const INVITATION_ID: Uuid = uuid!("93000000-0000-0000-0000-000000000003");
const ADMIN_ID: Uuid = uuid!("93000000-0000-0000-0000-000000000010");
const OUTSIDER_ID: Uuid = uuid!("93000000-0000-0000-0000-000000000011");
const PLAYER_USER_ID: Uuid = uuid!("93000000-0000-0000-0000-000000000012");
const UNLINKED_USER_ID: Uuid = uuid!("93000000-0000-0000-0000-000000000013");
const PLAYER_ID: Uuid = uuid!("93000000-0000-0000-0000-000000000020");

struct Seed {
    invitation_token: String,
}

async fn seed(pool: &PgPool, max_uses: Option<i32>) -> Seed {
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index)
        VALUES ('93000000-0000-0000-0000-000000000020', 'Linked player', 9.4);
        INSERT INTO users (id, username, display_name, role, player_id) VALUES
        ('93000000-0000-0000-0000-000000000010', 'invite_admin', 'Admin', 'viewer', NULL),
        ('93000000-0000-0000-0000-000000000011', 'invite_outsider', 'Outsider', 'viewer', NULL),
        ('93000000-0000-0000-0000-000000000012', 'invite_linked', 'Linked', 'player',
         '93000000-0000-0000-0000-000000000020'),
        ('93000000-0000-0000-0000-000000000013', 'invite_unlinked', 'Unlinked', 'viewer', NULL);
        INSERT INTO tournaments
          (id, name, start_date, end_date, number_of_rounds, status) VALUES
        ('93000000-0000-0000-0000-000000000001', 'Invitation trip', '2026-09-01', '2026-09-03', 1, 'draft'),
        ('93000000-0000-0000-0000-000000000002', 'Other trip', '2026-10-01', '2026-10-03', 1, 'draft');
        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
        ('93000000-0000-0000-0000-000000000001', '93000000-0000-0000-0000-000000000010', 'admin'),
        ('93000000-0000-0000-0000-000000000002', '93000000-0000-0000-0000-000000000011', 'admin');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    let invitation_token = generate_invitation_token().unwrap();
    sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at,
            max_uses, series_id)
         VALUES ($1, $2, $3, $4, now() + interval '1 day', $5, $1)",
    )
    .bind(INVITATION_ID)
    .bind(TOURNAMENT_ID)
    .bind(hash_invitation_token(&invitation_token).as_slice())
    .bind(ADMIN_ID)
    .bind(max_uses)
    .execute(pool)
    .await
    .unwrap();
    for (user_id, token) in [
        (ADMIN_ID, "admin-invitation-session"),
        (OUTSIDER_ID, "outsider-invitation-session"),
        (PLAYER_USER_ID, "player-invitation-session"),
        (UNLINKED_USER_ID, "unlinked-invitation-session"),
    ] {
        auth::create_session(
            pool,
            user_id,
            &hash_session_token(token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
    }
    Seed { invitation_token }
}

fn json_request(builder: Builder, value: Value) -> Request<Body> {
    builder
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}

fn authorized(builder: Builder, token: &str) -> Builder {
    builder
        .header(header::COOKIE, format!("golf_session={token}"))
        .header("x-csrf-token", derive_csrf_token(token))
}

async fn body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn post_token(app: axum::Router, path: String, token: &str) -> axum::response::Response {
    app.oneshot(json_request(Request::post(path), json!({"token": token})))
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn preview_is_minimal_and_wrong_tokens_are_one_oracle_safe_error(pool: PgPool) {
    let seed = seed(&pool, Some(1)).await;
    let app = api::router(AppState::new(pool.clone()));
    let valid = post_token(
        app.clone(),
        format!("/api/invitations/{INVITATION_ID}/preview"),
        &seed.invitation_token,
    )
    .await;
    assert_eq!(valid.status(), StatusCode::OK);
    assert_eq!(valid.headers()[header::CACHE_CONTROL], "no-store");
    let valid_body = body(valid).await;
    assert_eq!(valid_body["tournament"]["id"], TOURNAMENT_ID.to_string());
    assert_eq!(valid_body["tournament"]["name"], "Invitation trip");
    assert!(valid_body["tournament"].get("description").is_none());
    assert!(valid_body["invitation"].get("token").is_none());

    let wrong_token = generate_invitation_token().unwrap();
    let baseline = post_token(
        app.clone(),
        format!("/api/invitations/{INVITATION_ID}/preview"),
        &wrong_token,
    )
    .await;
    assert_eq!(baseline.status(), StatusCode::NOT_FOUND);
    let baseline_body = body(baseline).await;
    assert_eq!(baseline_body["error"]["code"], "invitation_invalid");
    let wrong_unknown = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/invitations/{INVITATION_ID}/preview")),
            json!({"token": wrong_token, "unexpected": true}),
        ))
        .await
        .unwrap();
    assert_eq!(wrong_unknown.status(), StatusCode::NOT_FOUND);
    assert_eq!(wrong_unknown.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(body(wrong_unknown).await, baseline_body);
    let correct_unknown = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/invitations/{INVITATION_ID}/preview")),
            json!({"token": seed.invitation_token, "unexpected": true}),
        ))
        .await
        .unwrap();
    assert_eq!(correct_unknown.status(), StatusCode::BAD_REQUEST);
    assert_eq!(correct_unknown.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        body(correct_unknown).await["error"]["code"],
        "validation_error"
    );

    sqlx::query(
        "UPDATE tournament_invitations
         SET revoked_at = now(), revoked_by_user_id = $2 WHERE id = $1",
    )
    .bind(INVITATION_ID)
    .bind(ADMIN_ID)
    .execute(&pool)
    .await
    .unwrap();
    let revoked_wrong = post_token(
        app.clone(),
        format!("/api/invitations/{INVITATION_ID}/preview"),
        &wrong_token,
    )
    .await;
    assert_eq!(revoked_wrong.status(), StatusCode::NOT_FOUND);
    assert_eq!(body(revoked_wrong).await, baseline_body);
    let revoked_valid = post_token(
        app.clone(),
        format!("/api/invitations/{INVITATION_ID}/preview"),
        &seed.invitation_token,
    )
    .await;
    assert_eq!(revoked_valid.status(), StatusCode::GONE);
    assert_eq!(
        body(revoked_valid).await["error"]["code"],
        "invitation_revoked"
    );

    let expired_id = Uuid::new_v4();
    let expired_token = generate_invitation_token().unwrap();
    sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, created_at,
            expires_at, series_id)
         VALUES ($1, $2, $3, $4, now() - interval '2 days',
                 now() - interval '1 day', $1)",
    )
    .bind(expired_id)
    .bind(TOURNAMENT_ID)
    .bind(hash_invitation_token(&expired_token).as_slice())
    .bind(ADMIN_ID)
    .execute(&pool)
    .await
    .unwrap();
    let exhausted_id = Uuid::new_v4();
    let exhausted_token = generate_invitation_token().unwrap();
    sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at,
            max_uses, series_id)
         VALUES ($1, $2, $3, $4, now() + interval '1 day', 1, $1)",
    )
    .bind(exhausted_id)
    .bind(TOURNAMENT_ID)
    .bind(hash_invitation_token(&exhausted_token).as_slice())
    .bind(ADMIN_ID)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(
        r#"
        INSERT INTO tournament_memberships (tournament_id, user_id, role)
        VALUES ('93000000-0000-0000-0000-000000000001',
                '93000000-0000-0000-0000-000000000012', 'player');
        INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
        VALUES ('93000000-0000-0000-0000-000000000001',
                '93000000-0000-0000-0000-000000000020', 9.4);
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO invitation_redemptions
           (id, invitation_id, series_id, tournament_id, user_id, player_id, mode)
         VALUES ($1, $2, $2, $3, $4, $5, 'acceptance')",
    )
    .bind(Uuid::new_v4())
    .bind(exhausted_id)
    .bind(TOURNAMENT_ID)
    .bind(PLAYER_USER_ID)
    .bind(PLAYER_ID)
    .execute(&pool)
    .await
    .unwrap();

    for invitation_id in [expired_id, exhausted_id] {
        let wrong = post_token(
            app.clone(),
            format!("/api/invitations/{invitation_id}/preview"),
            &wrong_token,
        )
        .await;
        assert_eq!(wrong.status(), StatusCode::NOT_FOUND);
        assert_eq!(body(wrong).await, baseline_body);
    }
    let expired = post_token(
        app.clone(),
        format!("/api/invitations/{expired_id}/preview"),
        &expired_token,
    )
    .await;
    assert_eq!(expired.status(), StatusCode::GONE);
    assert_eq!(body(expired).await["error"]["code"], "invitation_expired");
    let exhausted = post_token(
        app.clone(),
        format!("/api/invitations/{exhausted_id}/preview"),
        &exhausted_token,
    )
    .await;
    assert_eq!(exhausted.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(exhausted).await["error"]["code"],
        "invitation_exhausted"
    );

    for (path, token) in [
        ("/api/invitations/not-a-uuid/preview", wrong_token.as_str()),
        ("/api/invitations/preview", wrong_token.as_str()),
        (
            "/api/invitations/93000000-0000-0000-0000-000000000099/preview",
            wrong_token.as_str(),
        ),
        (
            "/api/invitations/93000000-0000-0000-0000-000000000099/preview",
            "malformed",
        ),
    ] {
        let response = post_token(app.clone(), path.to_owned(), token).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(body(response).await, baseline_body);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn registration_is_atomic_sets_session_and_rejects_identity_conflicts(pool: PgPool) {
    let seed = seed(&pool, None).await;
    let app = api::router(AppState::new(pool.clone()));
    let request = json!({
        "token": seed.invitation_token,
        "account": {"username": " New_Player ", "password": "a secure test password"},
        "player": {"display_name": " New player ", "handicap_index": 12.3}
    });
    let wrong_token = generate_invitation_token().unwrap();
    let wrong_malformed = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/invitations/{INVITATION_ID}/register")),
            json!({"token": wrong_token, "account": "bad", "unexpected": true}),
        ))
        .await
        .unwrap();
    assert_eq!(wrong_malformed.status(), StatusCode::NOT_FOUND);
    assert_eq!(wrong_malformed.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        body(wrong_malformed).await["error"]["code"],
        "invitation_invalid"
    );
    let correct_malformed = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/invitations/{INVITATION_ID}/register")),
            json!({"token": seed.invitation_token, "account": "bad", "unexpected": true}),
        ))
        .await
        .unwrap();
    assert_eq!(correct_malformed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        correct_malformed.headers()[header::CACHE_CONTROL],
        "no-store"
    );
    assert_eq!(
        body(correct_malformed).await["error"]["code"],
        "validation_error"
    );
    let response = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/invitations/{INVITATION_ID}/register"))
                .header(header::COOKIE, "golf_session=stale-session"),
            request.clone(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(response.headers().contains_key(header::SET_COOKIE));
    let response_body = body(response).await;
    let user_id = Uuid::parse_str(response_body["session"]["user_id"].as_str().unwrap()).unwrap();
    let player_id = Uuid::parse_str(response_body["player_id"].as_str().unwrap()).unwrap();
    let facts = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
        "SELECT
           (SELECT count(*) FROM users WHERE id = $1 AND player_id = $2 AND username = 'new_player'),
           (SELECT count(*) FROM handicap_history WHERE player_id = $2 AND changed_by = $1),
           (SELECT count(*) FROM tournament_memberships WHERE tournament_id = $3 AND user_id = $1 AND role = 'player'),
           (SELECT count(*) FROM tournament_players WHERE tournament_id = $3 AND player_id = $2 AND status = 'active'),
           (SELECT count(*) FROM tournament_handicap_history WHERE tournament_id = $3 AND player_id = $2 AND changed_by = $1),
           (SELECT count(*) FROM invitation_redemptions WHERE tournament_id = $3 AND user_id = $1 AND player_id = $2 AND mode = 'registration')",
    )
    .bind(user_id)
    .bind(player_id)
    .bind(TOURNAMENT_ID)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(facts, (1, 1, 1, 1, 1, 1));

    let before = sqlx::query_as::<_, (i64, i64)>(
        "SELECT (SELECT count(*) FROM players), (SELECT count(*) FROM users)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let duplicate = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/invitations/{INVITATION_ID}/register")),
            request,
        ))
        .await
        .unwrap();
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(duplicate).await["error"]["code"],
        "username_already_registered"
    );
    assert_eq!(
        sqlx::query_as::<_, (i64, i64)>(
            "SELECT (SELECT count(*) FROM players), (SELECT count(*) FROM users)",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        before
    );

    let wrong = generate_invitation_token().unwrap();
    let active_wrong = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/invitations/{INVITATION_ID}/register"))
                .header(header::COOKIE, "golf_session=player-invitation-session"),
            json!({
                "token": wrong,
                "account": {"username": "other_player", "password": "a secure test password"},
                "player": {"display_name": "Other", "handicap_index": 4.0}
            }),
        ))
        .await
        .unwrap();
    assert_eq!(active_wrong.status(), StatusCode::NOT_FOUND);

    let active_valid = app
        .oneshot(json_request(
            Request::post(format!("/api/invitations/{INVITATION_ID}/register"))
                .header(header::COOKIE, "golf_session=player-invitation-session"),
            json!({
                "token": seed.invitation_token,
                "account": {"username": "other_player", "password": "a secure test password"},
                "player": {"display_name": "Other", "handicap_index": 4.0}
            }),
        ))
        .await
        .unwrap();
    assert_eq!(active_valid.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(active_valid).await["error"]["code"],
        "already_authenticated"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn acceptance_uses_exact_player_and_recovers_idempotently(pool: PgPool) {
    let seed = seed(&pool, Some(1)).await;
    let app = api::router(AppState::new(pool.clone()));
    let path = format!("/api/invitations/{INVITATION_ID}/accept");
    let accept_request = || {
        json_request(
            authorized(Request::post(&path), "player-invitation-session"),
            json!({"token": seed.invitation_token.clone()}),
        )
    };
    let wrong_token = generate_invitation_token().unwrap();
    let wrong_unknown = app
        .clone()
        .oneshot(json_request(
            authorized(Request::post(&path), "player-invitation-session"),
            json!({"token": wrong_token, "unexpected": true}),
        ))
        .await
        .unwrap();
    assert_eq!(wrong_unknown.status(), StatusCode::NOT_FOUND);
    assert_eq!(wrong_unknown.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        body(wrong_unknown).await["error"]["code"],
        "invitation_invalid"
    );
    let correct_unknown = app
        .clone()
        .oneshot(json_request(
            authorized(Request::post(&path), "player-invitation-session"),
            json!({"token": seed.invitation_token, "unexpected": true}),
        ))
        .await
        .unwrap();
    assert_eq!(correct_unknown.status(), StatusCode::BAD_REQUEST);
    assert_eq!(correct_unknown.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        body(correct_unknown).await["error"]["code"],
        "validation_error"
    );
    let joined = app.clone().oneshot(accept_request()).await.unwrap();
    assert_eq!(joined.status(), StatusCode::CREATED);
    assert_eq!(body(joined).await["status"], "joined");
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT role::text FROM tournament_memberships
             WHERE tournament_id = $1 AND user_id = $2",
        )
        .bind(TOURNAMENT_ID)
        .bind(PLAYER_USER_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "player"
    );

    sqlx::query(
        "UPDATE tournament_invitations
         SET revoked_at = now(), revoked_by_user_id = $2 WHERE id = $1",
    )
    .bind(INVITATION_ID)
    .bind(ADMIN_ID)
    .execute(&pool)
    .await
    .unwrap();
    let replay = app.clone().oneshot(accept_request()).await.unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(body(replay).await["status"], "already_joined");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM invitation_redemptions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );

    sqlx::query("UPDATE players SET active = FALSE WHERE id = $1")
        .bind(PLAYER_ID)
        .execute(&pool)
        .await
        .unwrap();
    let inactive = app.clone().oneshot(accept_request()).await.unwrap();
    assert_eq!(inactive.status(), StatusCode::CONFLICT);
    assert_eq!(body(inactive).await["error"]["code"], "player_inactive");

    let wrong_token = generate_invitation_token().unwrap();
    let wrong_unlinked = app
        .clone()
        .oneshot(json_request(
            authorized(Request::post(&path), "unlinked-invitation-session"),
            json!({"token": wrong_token}),
        ))
        .await
        .unwrap();
    assert_eq!(wrong_unlinked.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body(wrong_unlinked).await["error"]["code"],
        "invitation_invalid"
    );
    sqlx::query("UPDATE players SET email = 'unlinked@invite.test' WHERE id = $1")
        .bind(PLAYER_ID)
        .execute(&pool)
        .await
        .unwrap();
    let valid_unlinked = app
        .oneshot(json_request(
            authorized(Request::post(path), "unlinked-invitation-session"),
            json!({"token": seed.invitation_token}),
        ))
        .await
        .unwrap();
    assert_eq!(valid_unlinked.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(valid_unlinked).await["error"]["code"],
        "account_player_required"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn redemption_guard_lifecycle_errors_keep_the_api_contract(pool: PgPool) {
    let seed = seed(&pool, None).await;
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION test_force_closed_before_redemption() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
            UPDATE tournaments SET status = 'completed' WHERE id = NEW.tournament_id;
            RETURN NEW;
        END;
        $$;
        CREATE TRIGGER aaa_test_force_closed_before_redemption
        BEFORE INSERT ON invitation_redemptions
        FOR EACH ROW EXECUTE FUNCTION test_force_closed_before_redemption();
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = api::router(AppState::new(pool.clone()));
    let response = app
        .oneshot(json_request(
            authorized(
                Request::post(format!("/api/invitations/{INVITATION_ID}/accept")),
                "player-invitation-session",
            ),
            json!({"token": seed.invitation_token}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        body(response).await["error"]["code"],
        "tournament_not_joinable"
    );
    assert_eq!(
        sqlx::query_as::<_, (String, i64, i64, i64)>(
            "SELECT status::text,
                    (SELECT count(*) FROM tournament_memberships WHERE user_id = $1),
                    (SELECT count(*) FROM tournament_players WHERE player_id = $2),
                    (SELECT count(*) FROM invitation_redemptions)
             FROM tournaments WHERE id = $3",
        )
        .bind(PLAYER_USER_ID)
        .bind(PLAYER_ID)
        .bind(TOURNAMENT_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        ("draft".to_owned(), 0, 0, 0)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn tournament_admin_can_issue_list_rotate_and_idempotently_revoke(pool: PgPool) {
    seed(&pool, None).await;
    let app = api::router(AppState::new(pool.clone()));
    let collection = format!("/api/tournaments/{TOURNAMENT_ID}/invitations");
    let expires_at = Utc::now() + Duration::days(2);
    let no_csrf = app
        .clone()
        .oneshot(json_request(
            Request::post(&collection)
                .header(header::COOKIE, "golf_session=admin-invitation-session"),
            json!({"expires_at": expires_at, "max_uses": 3}),
        ))
        .await
        .unwrap();
    assert_eq!(no_csrf.status(), StatusCode::FORBIDDEN);
    assert_eq!(no_csrf.headers()[header::CACHE_CONTROL], "no-store");

    let issued = app
        .clone()
        .oneshot(json_request(
            authorized(Request::post(&collection), "admin-invitation-session"),
            json!({"expires_at": expires_at, "max_uses": 3}),
        ))
        .await
        .unwrap();
    assert_eq!(issued.status(), StatusCode::CREATED);
    let issued_body = body(issued).await;
    let issued_id = issued_body["id"].as_str().unwrap();
    assert_eq!(issued_body["series_id"], issued_id);
    assert_eq!(issued_body["max_uses"], 3);
    assert_eq!(issued_body["token"].as_str().unwrap().len(), 43);

    let list = app
        .clone()
        .oneshot(
            Request::get(&collection)
                .header(header::COOKIE, "golf_session=admin-invitation-session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let list_body = body(list).await;
    assert_eq!(list_body.as_array().unwrap().len(), 2);
    assert!(list_body[0].get("token").is_none());
    assert!(list_body[0].get("token_hash").is_none());

    let rotate_path = format!("/api/tournaments/{TOURNAMENT_ID}/invitations/{issued_id}/rotate");
    let rotated = app
        .clone()
        .oneshot(json_request(
            authorized(Request::post(&rotate_path), "admin-invitation-session"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(rotated.status(), StatusCode::CREATED);
    let rotated_body = body(rotated).await;
    assert_eq!(rotated_body["predecessor_id"], issued_id);
    assert_eq!(rotated_body["series_id"], issued_id);
    assert_eq!(rotated_body["expires_at"], issued_body["expires_at"]);
    assert_eq!(rotated_body["max_uses"], 3);
    let successor_id = rotated_body["id"].as_str().unwrap();

    let revoke_path = format!("/api/tournaments/{TOURNAMENT_ID}/invitations/{successor_id}");
    for _ in 0..2 {
        let revoked = app
            .clone()
            .oneshot(
                authorized(Request::delete(&revoke_path), "admin-invitation-session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
        assert_eq!(revoked.headers()[header::CACHE_CONTROL], "no-store");
    }
    let provenance = sqlx::query_as::<_, (Option<Uuid>, bool)>(
        "SELECT revoked_by_user_id, revocation_actor_known
         FROM tournament_invitations WHERE id = $1",
    )
    .bind(Uuid::parse_str(successor_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(provenance, (Some(ADMIN_ID), true));

    let denied = app
        .oneshot(json_request(
            authorized(Request::post(collection), "outsider-invitation-session"),
            json!({"expires_at": expires_at, "max_uses": null}),
        ))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
}
