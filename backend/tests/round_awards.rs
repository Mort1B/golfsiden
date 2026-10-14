#![cfg(feature = "database-tests")]
mod fantasy_results_support;
use axum::http::header;
use fantasy_results_support::*;
use golf_api::{
    domain::{leaderboards::LeaderboardMetric, round_awards::AwardType},
    repositories::{
        leaderboards,
        round_awards::{self, Command, Error as AwardError},
        round_completion,
    },
};
fn command(player: Uuid, kind: AwardType) -> Command {
    Command {
        version: 1,
        request_id: Uuid::new_v4(),
        award_type: kind,
        player_id: Some(player),
        expected_revision: "0".into(),
        reason: "Measured winner".into(),
        correction: false,
    }
}
async fn completed(pool: &PgPool, final_round: bool) -> (Fantasy, Uuid) {
    let f = fixture(pool).await;
    let r = f.rounds[if final_round { 2 } else { 0 }];
    fantasy::save(pool, f.base.session, f.base.tournament, r, &lineup(&f))
        .await
        .unwrap();
    close(pool, &f, r).await;
    source_fixture(pool, &f, r).await;
    for p in &f.players {
        fill(pool, &f, r, ScoreOwner::Player { id: *p }, 4).await;
    }
    round_completion::complete_authorized(pool, f.base.session, r)
        .await
        .unwrap();
    (f, r)
}
async fn board(pool: &PgPool, user: Uuid, r: Uuid) -> Value {
    serde_json::to_value(
        leaderboards::round_for_member(pool, user, r, LeaderboardMetric::Net)
            .await
            .unwrap(),
    )
    .unwrap()
}
#[sqlx::test(migrations = "../migrations")]
async fn awards_replay_correction_removal_preserve_fantasy_and_native_scores(pool: PgPool) {
    let (f, r) = completed(&pool, false).await;
    let fantasy_before = scores(&pool, &f, r, f.base.session).await;
    let source_before = fantasy::source(
        &pool,
        f.base.session,
        f.base.tournament,
        r,
        OwnerKind::Player,
        f.players[0],
    )
    .await
    .unwrap()
    .source_token;
    let before = board(&pool, f.base.admin, r).await;
    let gross_before = serde_json::to_value(
        leaderboards::round_for_member(&pool, f.base.admin, r, LeaderboardMetric::Gross)
            .await
            .unwrap(),
    )
    .unwrap();
    let input = command(f.players[0], AwardType::ClosestToPin);
    let (_, receipt, changed) = round_awards::write(&pool, f.base.session, r, &input)
        .await
        .unwrap();
    assert!(changed);
    let (_, replayed, changed) = round_awards::write(&pool, f.base.session, r, &input)
        .await
        .unwrap();
    assert!(!changed);
    assert_eq!(
        serde_json::to_value(receipt).unwrap(),
        serde_json::to_value(replayed).unwrap()
    );
    let after = board(&pool, f.base.admin, r).await;
    let winner = after["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["owner"]["id"] == f.players[0].to_string())
        .unwrap();
    assert_eq!(winner["net_total"], 59);
    assert_eq!(winner["gross_total"], 72);
    assert_eq!(winner["net_awards"]["base_net_total"], 60);
    assert_eq!(winner["position"], 1);
    assert!(!winner["tied"].as_bool().unwrap());
    assert!(
        before["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["tied"] == true)
    );
    assert_eq!(
        serde_json::to_value(
            leaderboards::round_for_member(&pool, f.base.admin, r, LeaderboardMetric::Gross)
                .await
                .unwrap()
        )
        .unwrap(),
        gross_before
    );
    let overall = leaderboards::tournament_for_member(
        &pool,
        f.base.other,
        f.base.tournament,
        LeaderboardMetric::Net,
    )
    .await
    .unwrap();
    let public = serde_json::to_value(golf_api::domain::result_sharing::project(
        "Awards".into(),
        overall,
    ))
    .unwrap();
    assert_eq!(public["entries"][0]["total"], 59);
    for entry in public["entries"].as_array().unwrap() {
        assert_eq!(entry.as_object().unwrap().len(), 12);
        for private in [
            "net_awards",
            "player_id",
            "actor_id",
            "reason",
            "request_id",
            "contributions",
        ] {
            assert!(entry.get(private).is_none());
        }
    }
    assert_eq!(scores(&pool, &f, r, f.base.session).await, fantasy_before);
    assert_eq!(
        fantasy::source(
            &pool,
            f.base.session,
            f.base.tournament,
            r,
            OwnerKind::Player,
            f.players[0]
        )
        .await
        .unwrap()
        .source_token,
        source_before
    );
    let mut altered = input.clone();
    altered.reason = "Different request".into();
    assert!(matches!(
        round_awards::write(&pool, f.base.session, r, &altered).await,
        Err(AwardError::Request)
    ));
    altered.request_id = Uuid::new_v4();
    assert!(matches!(
        round_awards::write(&pool, f.base.session, r, &altered).await,
        Err(AwardError::Revision)
    ));
    let drive = command(f.players[0], AwardType::LongestDrive);
    round_awards::write(&pool, f.base.session, r, &drive)
        .await
        .unwrap();
    let mut replacement = command(f.players[1], AwardType::ClosestToPin);
    replacement.expected_revision = "1".into();
    assert!(matches!(
        round_awards::write(&pool, f.base.session, r, &replacement).await,
        Err(AwardError::Correction)
    ));
    replacement.correction = true;
    round_awards::write(&pool, f.base.session, r, &replacement)
        .await
        .unwrap();
    let mut removal = replacement.clone();
    removal.request_id = Uuid::new_v4();
    removal.expected_revision = "2".into();
    removal.player_id = None;
    round_awards::write(&pool, f.base.session, r, &removal)
        .await
        .unwrap();
    let view = round_awards::read(&pool, f.base.session, r).await.unwrap();
    assert_eq!(view.history.len(), 4);
    // Database guards independently reject unauthenticated audit insertion and a foreign preserved-field identity.
    let mut invalid = command(Uuid::new_v4(), AwardType::ClosestToPin);
    invalid.expected_revision = "3".into();
    invalid.correction = true;
    let insert = "INSERT INTO round_award_history(request_id,round_id,award_type,player_id,previous_player_id,revision,reason,correction,actor_id,command) VALUES($1,$2,'closest_to_pin',$3,NULL,4,'Measured winner',true,$4,$5)";
    let error = sqlx::query(insert)
        .bind(invalid.request_id)
        .bind(r)
        .bind(invalid.player_id)
        .bind(f.base.admin)
        .bind(serde_json::to_value(&invalid).unwrap())
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23514")
    );
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.round_award_session_id',$1,true)")
        .bind(f.base.session.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let error = sqlx::query(insert)
        .bind(invalid.request_id)
        .bind(r)
        .bind(invalid.player_id)
        .bind(f.base.admin)
        .bind(serde_json::to_value(&invalid).unwrap())
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23503")
    );
    tx.rollback().await.unwrap();
    assert_eq!(view.awards[0].player_id, None);
    assert_eq!(view.awards[1].player_id, Some(f.players[0]));
    assert_ne!(board(&pool, f.base.admin, r).await, before);
    assert!(
        sqlx::query("DELETE FROM round_award_history WHERE round_id=$1")
            .bind(r)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE round_awards SET player_id=$2 WHERE round_id=$1")
            .bind(r)
            .bind(f.players[2])
            .execute(&pool)
            .await
            .is_err()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn authorization_state_foreign_player_and_locked_correction(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let input = command(f.players[0], AwardType::ClosestToPin);
    assert!(matches!(
        round_awards::read(&pool, f.member_session, r).await,
        Err(AwardError::Authorization(_))
    ));
    assert!(matches!(
        round_awards::write(&pool, f.member_session, r, &input).await,
        Err(AwardError::Authorization(_))
    ));
    assert_eq!(
        round_awards::read(&pool, f.base.session, r)
            .await
            .unwrap()
            .availability,
        "round_not_completed"
    );
    assert!(matches!(
        round_awards::write(&pool, f.base.session, r, &input).await,
        Err(AwardError::State)
    ));
    source_fixture(&pool, &f, r).await;
    assert!(matches!(
        round_awards::write(&pool, f.base.session, r, &input).await,
        Err(AwardError::State)
    ));
    for p in &f.players {
        fill(&pool, &f, r, ScoreOwner::Player { id: *p }, 4).await;
    }
    round_completion::complete_authorized(&pool, f.base.session, r)
        .await
        .unwrap();
    let foreign = command(Uuid::new_v4(), AwardType::ClosestToPin);
    assert!(matches!(
        round_awards::write(&pool, f.base.session, r, &foreign).await,
        Err(AwardError::Invalid)
    ));
    round_completion::lock_authorized(&pool, f.base.session, r)
        .await
        .unwrap();
    assert!(matches!(
        round_awards::write(&pool, f.base.session, r, &input).await,
        Err(AwardError::Correction)
    ));
    let mut correction = input;
    correction.correction = true;
    round_awards::write(&pool, f.base.session, r, &correction)
        .await
        .unwrap();
    assert!(
        scorecards::save_authenticated(
            &pool,
            scorecards::AuthenticatedSaveScore {
                round_id: r,
                hole_id: holes(&pool, r).await[0],
                owner: ScoreOwner::Player { id: f.players[0] },
                gross_strokes: 5,
                session_id: f.base.session
            }
        )
        .await
        .is_err()
    );
    // Owner-only fixture constructs unsupported format; the operation itself sees normal guards.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("ALTER TABLE rounds DISABLE TRIGGER USER")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET scoring_format='singles_match_play' WHERE id=$1")
        .bind(f.rounds[1])
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE rounds ENABLE TRIGGER USER")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        round_awards::write(
            &pool,
            f.base.session,
            f.rounds[1],
            &command(f.players[0], AwardType::LongestDrive)
        )
        .await,
        Err(AwardError::Format)
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn hidden_final_and_concurrent_revision_are_safe(pool: PgPool) {
    let (f, r) = completed(&pool, true).await;
    hide(&pool, &f, true).await;
    let hidden = board(&pool, f.base.other, r).await;
    let first = command(f.players[0], AwardType::ClosestToPin);
    let second = command(f.players[1], AwardType::ClosestToPin);
    let other_admin = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES($1,'second_award_admin','Second Admin','player')").bind(other_admin).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(f.base.tournament)
    .bind(other_admin)
    .execute(&pool)
    .await
    .unwrap();
    let second_session = auth::create_session(
        &pool,
        other_admin,
        &hash_session_token("second-award-admin"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id;
    let (a, b) = tokio::join!(
        round_awards::write(&pool, f.base.session, r, &first),
        round_awards::write(&pool, second_session, r, &second)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(a, Err(AwardError::Revision)) || matches!(b, Err(AwardError::Revision)));
    assert_eq!(board(&pool, f.base.other, r).await, hidden);
    let overall = leaderboards::tournament_for_member(
        &pool,
        f.base.other,
        f.base.tournament,
        LeaderboardMetric::Net,
    )
    .await
    .unwrap();
    assert!(overall.entries.iter().all(|e| e.contributions.is_empty()));
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(r)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let other_pool = pool.clone();
    let session = f.base.session;
    let pending = command(f.players[0], AwardType::LongestDrive);
    let task =
        tokio::spawn(async move { round_awards::write(&other_pool, session, r, &pending).await });
    wait_for_lock(&pool, "%SELECT tournament_id FROM rounds%").await;
    auth::revoke_session(&pool, session).await.unwrap();
    blocker.commit().await.unwrap();
    assert!(matches!(
        task.await.unwrap(),
        Err(AwardError::Authorization(_))
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn endpoint_shape_csrf_no_store_and_definitive_conflicts(pool: PgPool) {
    let (f, r) = completed(&pool, false).await;
    let state = AppState::new(pool);
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let input = command(f.players[0], AwardType::ClosestToPin);
    let request = |method: &str, token: &str, csrf: bool| {
        let mut b = Request::builder()
            .method(method)
            .uri(format!("/api/rounds/{r}/awards"))
            .header(header::COOKIE, format!("golf_session={token}"))
            .header(header::CONTENT_TYPE, "application/json");
        if csrf {
            b = b.header("x-csrf-token", derive_csrf_token(token));
        }
        b.body(if method == "GET" {
            Body::empty()
        } else {
            Body::from(serde_json::to_vec(&input).unwrap())
        })
        .unwrap()
    };
    let response = app
        .clone()
        .oneshot(request("GET", ADMIN_TOKEN, false))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["awards"][0]["revision"], "0");
    assert_eq!(body["eligible_players"].as_array().unwrap().len(), 5);
    assert_eq!(
        app.clone()
            .oneshot(request("PUT", ADMIN_TOKEN, false))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = app
        .clone()
        .oneshot(request("PUT", ADMIN_TOKEN, true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["revision"], "1");
    assert_eq!(body["request_id"], input.request_id.to_string());
    let event = events.try_recv().unwrap();
    assert_eq!(event.resource, "round");
    assert_eq!(event.id, r);
    let replay = app
        .clone()
        .oneshot(request("PUT", ADMIN_TOKEN, true))
        .await
        .unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_body: Value =
        serde_json::from_slice(&replay.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(replay_body, body);
    assert!(events.try_recv().is_err());
    assert_eq!(
        app.oneshot(request("GET", "fantasy-member", false))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
}
