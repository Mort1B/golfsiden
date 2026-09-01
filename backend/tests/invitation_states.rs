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
use uuid::Uuid;

struct Base {
    tournament_id: Uuid,
    invitation_id: Uuid,
    admin_id: Uuid,
    invitation_token: String,
}

async fn seed_base(pool: &PgPool, max_uses: Option<i32>) -> Base {
    let tournament_id = Uuid::new_v4();
    let invitation_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, username, display_name, role)
         VALUES ($1, $2, 'Invitation admin', 'viewer')",
    )
    .bind(admin_id)
    .bind(admin_id.simple().to_string())
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds, status)
         VALUES ($1, 'State trip', '2026-09-01', '2026-09-03', 1, 'draft')",
    )
    .bind(tournament_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin')",
    )
    .bind(tournament_id)
    .bind(admin_id)
    .execute(pool)
    .await
    .unwrap();
    let invitation_token = generate_invitation_token().unwrap();
    sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at,
            max_uses, series_id)
         VALUES ($1, $2, $3, $4, now() + interval '2 days', $5, $1)",
    )
    .bind(invitation_id)
    .bind(tournament_id)
    .bind(hash_invitation_token(&invitation_token).as_slice())
    .bind(admin_id)
    .bind(max_uses)
    .execute(pool)
    .await
    .unwrap();
    auth::create_session(
        pool,
        admin_id,
        &hash_session_token("state-admin-session"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    Base {
        tournament_id,
        invitation_id,
        admin_id,
        invitation_token,
    }
}

struct LinkedUser {
    user_id: Uuid,
    player_id: Uuid,
    session_token: String,
}

async fn seed_linked_user(
    pool: &PgPool,
    base: &Base,
    label: &str,
    membership_role: Option<&str>,
    entrant_status: Option<&str>,
) -> LinkedUser {
    let user_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();
    let session_token = format!("{label}-state-session");
    sqlx::query(
        "INSERT INTO players (id, display_name, current_handicap_index)
         VALUES ($1, $2, 10.0)",
    )
    .bind(player_id)
    .bind(label)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, display_name, role, player_id)
         VALUES ($1, $2, $3, 'player', $4)",
    )
    .bind(user_id)
    .bind(user_id.simple().to_string())
    .bind(label)
    .bind(player_id)
    .execute(pool)
    .await
    .unwrap();
    if let Some(role) = membership_role {
        sqlx::query(
            "INSERT INTO tournament_memberships (tournament_id, user_id, role)
             VALUES ($1, $2, $3::tournament_role)",
        )
        .bind(base.tournament_id)
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
    }
    if let Some(status) = entrant_status {
        sqlx::query(
            "INSERT INTO tournament_players
               (tournament_id, player_id, tournament_handicap, status)
             VALUES ($1, $2, 10.0, $3::participant_status)",
        )
        .bind(base.tournament_id)
        .bind(player_id)
        .bind(status)
        .execute(pool)
        .await
        .unwrap();
    }
    auth::create_session(
        pool,
        user_id,
        &hash_session_token(&session_token),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    LinkedUser {
        user_id,
        player_id,
        session_token,
    }
}

fn authorized(builder: Builder, session_token: &str) -> Builder {
    builder
        .header(header::COOKIE, format!("golf_session={session_token}"))
        .header("x-csrf-token", derive_csrf_token(session_token))
}

fn json_request(builder: Builder, value: Value) -> Request<Body> {
    builder
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}

async fn accept(
    app: axum::Router,
    invitation_id: Uuid,
    invitation_token: &str,
    session_token: &str,
) -> axum::response::Response {
    app.oneshot(json_request(
        authorized(
            Request::post(format!("/api/invitations/{invitation_id}/accept")),
            session_token,
        ),
        json!({"token": invitation_token}),
    ))
    .await
    .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn acceptance_completes_missing_halves_and_preserves_roles(pool: PgPool) {
    let base = seed_base(&pool, None).await;
    let states = [
        ("admin", Some("admin"), None, "admin", StatusCode::CREATED),
        (
            "scorer",
            Some("scorer"),
            None,
            "scorer",
            StatusCode::CREATED,
        ),
        (
            "viewer",
            Some("viewer"),
            None,
            "player",
            StatusCode::CREATED,
        ),
        (
            "player-half",
            Some("player"),
            None,
            "player",
            StatusCode::CREATED,
        ),
        (
            "entrant-half",
            None,
            Some("active"),
            "player",
            StatusCode::CREATED,
        ),
    ];
    let app = api::router(AppState::new(pool.clone()));
    for (label, initial_role, entrant, expected_role, status) in states {
        let user = seed_linked_user(&pool, &base, label, initial_role, entrant).await;
        let response = accept(
            app.clone(),
            base.invitation_id,
            &base.invitation_token,
            &user.session_token,
        )
        .await;
        assert_eq!(response.status(), status, "{label}");
        if entrant.is_some() {
            let retry = accept(
                app.clone(),
                base.invitation_id,
                &base.invitation_token,
                &user.session_token,
            )
            .await;
            assert_eq!(retry.status(), StatusCode::OK, "{label} retry");
            assert_eq!(body(retry).await["status"], "already_joined");
        }
        let stored = sqlx::query_as::<_, (String, String)>(
            "SELECT tm.role::text, tp.status::text
             FROM tournament_memberships tm
             JOIN users u ON u.id = tm.user_id
             JOIN tournament_players tp
               ON tp.tournament_id = tm.tournament_id AND tp.player_id = u.player_id
             WHERE tm.tournament_id = $1 AND tm.user_id = $2",
        )
        .bind(base.tournament_id)
        .bind(user.user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, (expected_role.to_owned(), "active".to_owned()));
        let history = sqlx::query_as::<_, (f64, Option<Uuid>, Option<String>)>(
            "SELECT handicap_index::float8, changed_by, reason
             FROM tournament_handicap_history
             WHERE tournament_id = $1 AND player_id = $2
             ORDER BY effective_from, id",
        )
        .bind(base.tournament_id)
        .bind(user.player_id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            history,
            vec![(
                10.0,
                Some(user.user_id),
                Some(
                    if entrant.is_some() {
                        "invitation acceptance initial handicap repair"
                    } else {
                        "invitation join snapshot"
                    }
                    .to_owned()
                ),
            )],
            "{label}"
        );
        let redemption = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String)>(
            "SELECT tournament_id, user_id, player_id, mode::text
             FROM invitation_redemptions WHERE user_id = $1",
        )
        .bind(user.user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            redemption,
            (
                base.tournament_id,
                user.user_id,
                user.player_id,
                "acceptance".to_owned(),
            ),
            "{label}"
        );
    }

    let withdrawn =
        seed_linked_user(&pool, &base, "withdrawn", Some("player"), Some("withdrawn")).await;
    let response = accept(
        app,
        base.invitation_id,
        &base.invitation_token,
        &withdrawn.session_token,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(body(response).await["error"]["code"], "player_withdrawn");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM invitation_redemptions WHERE user_id = $1",
        )
        .bind(withdrawn.user_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM team_memberships")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flight_memberships")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn complete_join_retries_before_expired_rotated_and_exhausted_states(pool: PgPool) {
    let base = seed_base(&pool, Some(1)).await;
    let user = seed_linked_user(&pool, &base, "complete", Some("player"), Some("active")).await;
    sqlx::query(
        "INSERT INTO invitation_redemptions
           (id, invitation_id, series_id, tournament_id, user_id, player_id, mode)
         VALUES ($1, $2, $2, $3, $4, $5, 'acceptance')",
    )
    .bind(Uuid::new_v4())
    .bind(base.invitation_id)
    .bind(base.tournament_id)
    .bind(user.user_id)
    .bind(user.player_id)
    .execute(&pool)
    .await
    .unwrap();

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
    .bind(base.tournament_id)
    .bind(hash_invitation_token(&expired_token).as_slice())
    .bind(base.admin_id)
    .execute(&pool)
    .await
    .unwrap();

    let rotated_id = Uuid::new_v4();
    let rotated_token = generate_invitation_token().unwrap();
    sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at, series_id)
         VALUES ($1, $2, $3, $4, now() + interval '1 day', $1)",
    )
    .bind(rotated_id)
    .bind(base.tournament_id)
    .bind(hash_invitation_token(&rotated_token).as_slice())
    .bind(base.admin_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE tournament_invitations
         SET revoked_at = now(), revoked_by_user_id = $2 WHERE id = $1",
    )
    .bind(rotated_id)
    .bind(base.admin_id)
    .execute(&pool)
    .await
    .unwrap();
    let successor_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at,
            max_uses, series_id, predecessor_id)
         SELECT $1, tournament_id, $2, created_by_user_id, expires_at,
                max_uses, series_id, id
         FROM tournament_invitations WHERE id = $3",
    )
    .bind(successor_id)
    .bind(hash_invitation_token(&generate_invitation_token().unwrap()).as_slice())
    .bind(rotated_id)
    .execute(&pool)
    .await
    .unwrap();

    let app = api::router(AppState::new(pool.clone()));
    for (invitation_id, token) in [
        (base.invitation_id, base.invitation_token.as_str()),
        (expired_id, expired_token.as_str()),
        (rotated_id, rotated_token.as_str()),
    ] {
        let response = accept(app.clone(), invitation_id, token, &user.session_token).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body(response).await["status"], "already_joined");
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM invitation_redemptions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn partial_use_rotation_keeps_series_count_and_capacity(pool: PgPool) {
    let base = seed_base(&pool, Some(2)).await;
    let first = seed_linked_user(&pool, &base, "first", None, None).await;
    let second = seed_linked_user(&pool, &base, "second", None, None).await;
    let third = seed_linked_user(&pool, &base, "third", None, None).await;
    let app = api::router(AppState::new(pool.clone()));
    assert_eq!(
        accept(
            app.clone(),
            base.invitation_id,
            &base.invitation_token,
            &first.session_token,
        )
        .await
        .status(),
        StatusCode::CREATED
    );

    let rotate = app
        .clone()
        .oneshot(json_request(
            authorized(
                Request::post(format!(
                    "/api/tournaments/{}/invitations/{}/rotate",
                    base.tournament_id, base.invitation_id
                )),
                "state-admin-session",
            ),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(rotate.status(), StatusCode::CREATED);
    let rotate_body = body(rotate).await;
    assert_eq!(rotate_body["redemption_count"], 1);
    let successor_id = Uuid::parse_str(rotate_body["id"].as_str().unwrap()).unwrap();
    let successor_token = rotate_body["token"].as_str().unwrap();

    assert_eq!(
        accept(
            app.clone(),
            successor_id,
            successor_token,
            &second.session_token,
        )
        .await
        .status(),
        StatusCode::CREATED
    );
    let exhausted = accept(
        app.clone(),
        successor_id,
        successor_token,
        &third.session_token,
    )
    .await;
    assert_eq!(exhausted.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(exhausted).await["error"]["code"],
        "invitation_exhausted"
    );

    let listed = app
        .oneshot(
            Request::get(format!(
                "/api/tournaments/{}/invitations",
                base.tournament_id
            ))
            .header(header::COOKIE, "golf_session=state-admin-session")
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let list_body = body(listed).await;
    let series_rows: Vec<&Value> = list_body
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["series_id"] == base.invitation_id.to_string())
        .collect();
    assert_eq!(series_rows.len(), 2);
    assert!(series_rows.iter().all(|row| row["redemption_count"] == 2));
}
