#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Days, NaiveDate, Utc};
use golf_api::{
    AppState, api,
    auth::{hash_password, hash_session_token, verify_password},
    domain::{
        models::ScoringFormat,
        onboarding::{OnboardingInput, RoundInput, ValidatedOnboarding, validate},
    },
    repositories::onboarding::{
        self as onboarding_repository, CreateOnboardingParams, OnboardingRepositoryError,
    },
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn future_dates() -> (NaiveDate, NaiveDate) {
    let start = Utc::now()
        .date_naive()
        .checked_add_days(Days::new(2))
        .unwrap();
    let end = start.checked_add_days(Days::new(2)).unwrap();
    (start, end)
}

fn valid_request(username: &str) -> Value {
    let (start, end) = future_dates();
    json!({
        "creator": {
            "account": {"username": username, "password": "a secure test password"},
            "player": {"display_name": "Tournament Creator", "handicap_index": 12.3}
        },
        "tournament": {
            "name": "Annual Trip",
            "description": "Private tournament",
            "start_date": start,
            "end_date": end
        },
        "rounds": [
            {
                "round_number": 2,
                "name": "Foursomes",
                "round_date": start.checked_add_days(Days::new(1)).unwrap(),
                "scoring_format": "two_player_foursomes"
            },
            {
                "round_number": 1,
                "name": "Opening round",
                "round_date": start,
                "scoring_format": "individual_stroke_play"
            }
        ]
    })
}

fn repository_input(username: &str, tournament_name: &str) -> ValidatedOnboarding {
    let (start_date, end_date) = future_dates();
    validate(
        OnboardingInput {
            username: username.to_owned(),
            password: "a secure test password".to_owned(),
            display_name: tournament_name.to_owned(),
            handicap_index: 12.3,
            tournament_name: tournament_name.to_owned(),
            description: "Repository rollback test".to_owned(),
            start_date,
            end_date,
            rounds: vec![RoundInput {
                round_number: 1,
                name: "Opening round".to_owned(),
                round_date: start_date,
                scoring_format: ScoringFormat::IndividualStrokePlay,
            }],
        },
        Utc::now().date_naive(),
    )
    .unwrap()
}

async fn onboarding_table_counts(pool: &PgPool) -> Vec<(String, i64)> {
    sqlx::query_as(
        "SELECT relation, row_count FROM (
           SELECT 'handicap_history'::text AS relation, count(*) AS row_count FROM handicap_history
           UNION ALL SELECT 'players', count(*) FROM players
           UNION ALL SELECT 'rounds', count(*) FROM rounds
           UNION ALL SELECT 'tournament_handicap_history', count(*) FROM tournament_handicap_history
           UNION ALL SELECT 'tournament_invitations', count(*) FROM tournament_invitations
           UNION ALL SELECT 'tournament_memberships', count(*) FROM tournament_memberships
           UNION ALL SELECT 'tournament_players', count(*) FROM tournament_players
           UNION ALL SELECT 'tournaments', count(*) FROM tournaments
           UNION ALL SELECT 'user_sessions', count(*) FROM user_sessions
           UNION ALL SELECT 'users', count(*) FROM users
         ) counts ORDER BY relation",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn post_json(
    app: axum::Router,
    value: Value,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::post("/api/onboarding/tournaments")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    app.oneshot(builder.body(Body::from(value.to_string())).unwrap())
        .await
        .unwrap()
}

fn cookie_from(response: &axum::response::Response) -> String {
    response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

#[sqlx::test(migrations = "../migrations")]
async fn onboarding_atomically_links_creator_rounds_invite_and_session(pool: PgPool) {
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let (_, end_date) = future_dates();

    let response = post_json(
        app.clone(),
        valid_request(" Creator_Account "),
        Some("golf_session=stale-token"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let cookie = cookie_from(&response);
    let session_token = cookie.strip_prefix("golf_session=").unwrap();
    let body = response_json(response).await;

    assert_eq!(body["tournament"]["number_of_rounds"], 2);
    assert_eq!(body["tournament"]["scoring_mode"], "combined");
    assert_eq!(body["tournament"]["status"], "draft");
    assert_eq!(body["creator"]["tournament_role"], "admin");
    assert_eq!(body["session"]["role"], "player");
    assert_eq!(body["session"]["username"], "creator_account");
    assert_eq!(body["rounds"][0]["round_number"], 1);
    assert_eq!(body["rounds"][1]["round_number"], 2);
    for round in body["rounds"].as_array().unwrap() {
        assert_eq!(round["status"], "draft");
        assert_eq!(round["number_of_holes"], 18);
        assert_eq!(round["handicap_enabled"], true);
        let expected_allowance = if round["scoring_format"] == "two_player_foursomes" {
            50
        } else {
            100
        };
        assert_eq!(round["handicap_allowance_percent"], expected_allowance);
        assert!(round["course_id"].is_null());
        assert!(round["tee_id"].is_null());
        assert_eq!(round["course_name"], "");
        assert_eq!(round["tee_name"], "");
    }

    let user_id = Uuid::parse_str(body["creator"]["user_id"].as_str().unwrap()).unwrap();
    let player_id = Uuid::parse_str(body["creator"]["player_id"].as_str().unwrap()).unwrap();
    let tournament_id = Uuid::parse_str(body["tournament"]["id"].as_str().unwrap()).unwrap();
    assert_eq!(body["session"]["user_id"], user_id.to_string());
    assert_eq!(body["session"]["player_id"], player_id.to_string());

    let user = sqlx::query_as::<_, (String, String, String, Option<Uuid>)>(
        "SELECT username, role::text, password_hash, player_id FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(user.0, "creator_account");
    assert_eq!(user.1, "player");
    assert!(user.2.starts_with("$argon2"));
    assert_ne!(user.2, "a secure test password");
    assert!(verify_password("a secure test password".to_owned(), user.2).await);
    assert_eq!(user.3, Some(player_id));

    let player_email =
        sqlx::query_scalar::<_, Option<String>>("SELECT email FROM players WHERE id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(player_email.is_none());
    let membership_role = sqlx::query_scalar::<_, String>(
        "SELECT role::text FROM tournament_memberships WHERE tournament_id = $1 AND user_id = $2",
    )
    .bind(tournament_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(membership_role, "admin");

    let histories = sqlx::query_as::<_, (i64, i64)>(
        "SELECT
           (SELECT count(*) FROM handicap_history WHERE player_id = $1 AND changed_by = $2),
           (SELECT count(*) FROM tournament_handicap_history
             WHERE tournament_id = $3 AND player_id = $1 AND changed_by = $2)",
    )
    .bind(player_id)
    .bind(user_id)
    .bind(tournament_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(histories, (1, 1));

    let stored_session =
        sqlx::query_scalar::<_, Vec<u8>>("SELECT token_hash FROM user_sessions WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored_session, hash_session_token(session_token));
    assert_ne!(stored_session, session_token.as_bytes());

    let invite_token = body["invitation"]["token"].as_str().unwrap();
    assert_ne!(invite_token, session_token);
    let invitation = sqlx::query_as::<_, (Vec<u8>, chrono::DateTime<Utc>, Option<i32>)>(
        "SELECT token_hash, expires_at, max_uses FROM tournament_invitations
         WHERE id = $1 AND tournament_id = $2 AND created_by_user_id = $3",
    )
    .bind(Uuid::parse_str(body["invitation"]["id"].as_str().unwrap()).unwrap())
    .bind(tournament_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        invitation.0,
        golf_api::auth::hash_invitation_token(invite_token)
    );
    assert_ne!(invitation.0, invite_token.as_bytes());
    assert_eq!(
        invitation.1.date_naive(),
        end_date.checked_add_days(Days::new(7)).unwrap()
    );
    assert_eq!(invitation.1.time(), chrono::NaiveTime::MIN);
    assert_eq!(invitation.2, None);

    let current = app
        .clone()
        .oneshot(
            Request::get("/api/auth/session")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(current.status(), StatusCode::OK);
    let event = events.try_recv().unwrap();
    assert_eq!(event.resource, "tournament");
    assert_eq!(event.id, tournament_id);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn invalid_unknown_and_oversized_requests_write_nothing(pool: PgPool) {
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    let mut invalid = valid_request("invalid_user");
    invalid["rounds"][1]["round_number"] = json!(3);
    let mut unknown = valid_request("unknown_user");
    unknown["creator"]["player"]["role"] = json!("admin");
    let mut overlong_field = valid_request("field_user");
    overlong_field["tournament"]["description"] = json!("x".repeat(2_001));
    let mut unsupported_format = valid_request("format_user");
    unsupported_format["rounds"][0]["scoring_format"] = json!("stableford");
    let mut oversized = valid_request("oversized_user");
    oversized["tournament"]["description"] = json!("x".repeat(70_000));

    for input in [
        invalid,
        unknown,
        overlong_field,
        unsupported_format,
        oversized,
    ] {
        let response = post_json(app.clone(), input, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(!response.headers().contains_key(header::SET_COOKIE));
        assert_eq!(
            response_json(response).await["error"]["code"],
            "validation_error"
        );
    }
    let counts = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT (SELECT count(*) FROM players),
                (SELECT count(*) FROM users),
                (SELECT count(*) FROM tournaments)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(counts, (0, 0, 0));
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn duplicate_username_is_stable_and_rolls_back_prior_player_insert(pool: PgPool) {
    sqlx::query(
        "INSERT INTO users (id, username, display_name, role)
         VALUES ($1, 'creator_account', 'Existing', 'viewer')",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let response = post_json(api::router(state), valid_request(" CREATOR_ACCOUNT "), None).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(!response.headers().contains_key(header::SET_COOKIE));
    assert_eq!(
        response_json(response).await["error"]["code"],
        "username_already_registered"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM players")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournaments")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_normalized_username_registration_commits_exactly_once(pool: PgPool) {
    let app = api::router(AppState::new(pool.clone()));
    let first = tokio::spawn(post_json(
        app.clone(),
        valid_request(" Race_Account "),
        None,
    ));
    let second = tokio::spawn(post_json(app, valid_request("race_account"), None));
    let (first, second) = tokio::join!(first, second);
    let statuses = [first.unwrap().status(), second.unwrap().status()];
    assert!(statuses.contains(&StatusCode::CREATED));
    assert!(statuses.contains(&StatusCode::CONFLICT));
    assert_eq!(
        sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT
               (SELECT count(*) FROM users WHERE username = 'race_account'),
               (SELECT count(*) FROM players),
               (SELECT count(*) FROM tournaments)",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        (1, 1, 1)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn valid_authenticated_call_is_rejected_without_a_second_event(pool: PgPool) {
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let first = post_json(app.clone(), valid_request("first_user"), None).await;
    let cookie = cookie_from(&first);
    assert_eq!(first.status(), StatusCode::CREATED);
    let _ = events.try_recv().unwrap();

    let second = post_json(app, valid_request("second_user"), Some(&cookie)).await;
    assert_eq!(second.status(), StatusCode::CONFLICT);
    assert_eq!(second.headers()[header::CACHE_CONTROL], "no-store");
    assert!(!second.headers().contains_key(header::SET_COOKIE));
    assert_eq!(
        response_json(second).await["error"]["code"],
        "already_authenticated"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournaments")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn late_session_hash_collision_rolls_back_every_second_onboarding_row(pool: PgPool) {
    let first_input = repository_input("first_repository", "First repository trip");
    let second_input = repository_input("second_repository", "Second repository trip");
    let password_hash = hash_password(b"a secure test password").unwrap();
    let shared_session_hash = [7_u8; 32];
    let first_invitation_hash = [8_u8; 32];
    let second_invitation_hash = [9_u8; 32];
    let session_expires_at = Utc::now() + chrono::Duration::hours(1);

    onboarding_repository::create(
        &pool,
        CreateOnboardingParams {
            input: &first_input,
            password_hash: &password_hash,
            session_token_hash: &shared_session_hash,
            session_expires_at,
            invitation_token_hash: &first_invitation_hash,
        },
    )
    .await
    .unwrap();
    let before = onboarding_table_counts(&pool).await;
    assert!(before.iter().all(|(_, count)| *count == 1));

    let error = onboarding_repository::create(
        &pool,
        CreateOnboardingParams {
            input: &second_input,
            password_hash: &password_hash,
            session_token_hash: &shared_session_hash,
            session_expires_at,
            invitation_token_hash: &second_invitation_hash,
        },
    )
    .await
    .unwrap_err();
    match error {
        OnboardingRepositoryError::Database(sqlx::Error::Database(database)) => {
            assert!(database.is_unique_violation());
            assert_eq!(database.constraint(), Some("user_sessions_token_hash_key"));
        }
        other => panic!("unexpected repository error: {other:?}"),
    }

    assert_eq!(onboarding_table_counts(&pool).await, before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM users WHERE username = 'second_repository'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tournaments WHERE name = 'Second repository trip'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}
