use std::{sync::Arc, time::Duration};

use super::base::{self, id};
use axum::{Router, body::Body, http::Request};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    domain::{
        match_play::commands::{Command, Event, Request as MatchRequest},
        scorecards::{ScoreOwner, ScoreRevision},
    },
    repositories::{auth, match_play, scorecards},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use tower::ServiceExt;
use uuid::Uuid;

pub struct Fixture {
    pub state: Arc<AppState>,
    pub app: Router,
    pub session: Uuid,
    pub format: &'static str,
}

pub async fn fixture(pool: &PgPool, format: &'static str) -> Fixture {
    let initial_format = if format == "two_player_foursomes" {
        "individual_stroke_play"
    } else {
        format
    };
    let session = base::draft(pool, initial_format).await;
    if format == "two_player_foursomes" {
        sqlx::query("UPDATE rounds SET scoring_format='two_player_foursomes',handicap_allowance_percent=50 WHERE id=$1")
            .bind(id(5))
            .execute(pool)
            .await
            .unwrap();
        for (team, players) in [(21, [11, 12]), (22, [13, 14])] {
            sqlx::query("INSERT INTO teams(id,round_id,tournament_id,name,starting_hole) VALUES($1,$2,$3,$4,1)")
                .bind(id(team)).bind(id(5)).bind(id(2)).bind(format!("Pair {team}")).execute(pool).await.unwrap();
            for (index, player) in players.into_iter().enumerate() {
                sqlx::query("INSERT INTO team_memberships(team_id,round_id,tournament_id,player_id,display_order) VALUES($1,$2,$3,$4,$5)")
                    .bind(id(team)).bind(id(5)).bind(id(2)).bind(id(player)).bind(index as i16+1).execute(pool).await.unwrap();
            }
        }
    } else if format == "singles_match_play" {
        let updated = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
            .bind(id(5))
            .fetch_one(pool)
            .await
            .unwrap();
        match_play::setup::replace(
            pool,
            session,
            id(5),
            match_play::setup::Assignments {
                expected_round_updated_at: updated,
                matches: vec![
                    match_play::setup::Pair {
                        first_player_id: id(11),
                        second_player_id: id(12),
                    },
                    match_play::setup::Pair {
                        first_player_id: id(13),
                        second_player_id: id(14),
                    },
                ],
            },
        )
        .await
        .unwrap();
    }
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));
    Fixture {
        state,
        app,
        session,
        format,
    }
}

pub async fn new_session(pool: &PgPool, expire: bool) -> (Uuid, String) {
    let token = Uuid::new_v4().to_string();
    let session = auth::create_session(
        pool,
        id(1),
        &hash_session_token(&token),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap()
    .session_id;
    if expire {
        sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '5 seconds' WHERE id=$1")
            .bind(session).execute(pool).await.unwrap();
    }
    (session, token)
}

pub async fn request(app: Router, action: &str, token: &str) -> axum::response::Response {
    app.oneshot(
        Request::post(format!("/api/rounds/{}/{action}", id(5)))
            .header("cookie", format!("golf_session={token}"))
            .header("x-csrf-token", derive_csrf_token(token))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

pub async fn json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

pub async fn ready_to_complete(pool: &PgPool, f: &Fixture) {
    if f.format == "singles_match_play" {
        let matches: Vec<(Uuid,Uuid,i64)> = sqlx::query_as("SELECT id,second_player_id,revision FROM singles_matches WHERE round_id=$1 ORDER BY id")
            .bind(id(5)).fetch_all(pool).await.unwrap();
        for (m, conceding, revision) in matches {
            let ack = match_play::execute(
                pool,
                f.session,
                id(5),
                m,
                MatchRequest {
                    request_id: Uuid::new_v4(),
                    expected_revision: ScoreRevision::from_database(revision).unwrap(),
                    command: Command::Report {
                        event: Event::Concession {
                            conceding_player_id: conceding,
                            communicated: true,
                            after_hole: 0,
                        },
                    },
                },
            )
            .await
            .unwrap()
            .value;
            match_play::execute(
                pool,
                f.session,
                id(5),
                m,
                MatchRequest {
                    request_id: Uuid::new_v4(),
                    expected_revision: ack.applied_revision,
                    command: Command::Confirm {
                        result_agreed_or_awarded: true,
                    },
                },
            )
            .await
            .unwrap();
        }
    } else {
        let owners = if f.format == "two_player_foursomes" {
            vec![
                ScoreOwner::Team { id: id(21) },
                ScoreOwner::Team { id: id(22) },
            ]
        } else {
            (11..=14)
                .map(|p| ScoreOwner::Player { id: id(p) })
                .collect()
        };
        for owner in owners {
            for hole in 1..=18 {
                scorecards::save_authenticated(
                    pool,
                    scorecards::AuthenticatedSaveScore {
                        session_id: f.session,
                        round_id: id(5),
                        owner,
                        hole_id: id(100 + hole),
                        gross_strokes: 4,
                    },
                )
                .await
                .unwrap();
            }
            scorecards::confirm_authenticated(pool, id(5), owner, f.session)
                .await
                .unwrap();
        }
    }
}

pub async fn stored(pool: &PgPool) -> Value {
    sqlx::query_scalar(include_str!("state.sql"))
        .bind(id(5))
        .bind(id(2))
        .fetch_one(pool)
        .await
        .unwrap()
}

pub async fn wait_blocked(pool: &PgPool, pid: i32) {
    tokio::time::timeout(Duration::from_secs(5),async {
        loop {
            let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                .bind(pid).fetch_one(pool).await.unwrap();
            if blocked {break;}
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("lifecycle request must reach its controlled wait");
}

pub async fn wait_expired(pool: &PgPool, session: Uuid) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let expired: bool = sqlx::query_scalar(
                "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1",
            )
            .bind(session)
            .fetch_one(pool)
            .await
            .unwrap();
            if expired {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
    })
    .await
    .unwrap();
}

pub async fn gate(pool: &PgPool, late: bool) -> (Transaction<'_, Postgres>, i32) {
    let mut tx = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    if late {
        sqlx::query("SELECT pg_advisory_xact_lock(240014,2)")
            .execute(&mut *tx)
            .await
            .unwrap();
    } else {
        sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR UPDATE")
            .bind(id(2)).bind(id(1)).execute(&mut *tx).await.unwrap();
    }
    (tx, pid)
}

pub async fn install_late_gate(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION test_lifecycle_wait() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF (SELECT count(*) FROM round_handicap_snapshots WHERE round_id=NEW.id) <> 4 THEN
                RAISE EXCEPTION 'expected player snapshots before late wait';
            END IF;
            IF NEW.scoring_format='two_player_foursomes' AND
               (SELECT count(*) FROM round_team_handicap_snapshots WHERE round_id=NEW.id) <> 2 THEN
                RAISE EXCEPTION 'expected team snapshots before late wait';
            END IF;
            IF NOT EXISTS(SELECT 1 FROM tournament_handicap_locks WHERE tournament_id=NEW.tournament_id) THEN
                RAISE EXCEPTION 'expected tournament handicap lock before late wait';
            END IF;
            PERFORM pg_advisory_xact_lock(240014,2);
            RETURN NEW;
        END $$;
        CREATE TRIGGER test_lifecycle_wait AFTER UPDATE OF status ON rounds
            FOR EACH ROW WHEN(OLD.status IS DISTINCT FROM NEW.status)
            EXECUTE FUNCTION test_lifecycle_wait();
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
}
