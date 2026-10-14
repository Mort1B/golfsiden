use super::{base::id, support::*};
use axum::http::{StatusCode, header::CACHE_CONTROL};
use sqlx::PgPool;
use tokio::{
    sync::broadcast::error::TryRecvError,
    time::{Duration, timeout},
};

async fn cycle(pool: PgPool, format: &'static str, late: bool) {
    let f = fixture(&pool, format).await;
    if late {
        install_late_gate(&pool).await;
    }
    let mut events = f.state.live_events.subscribe();
    for (action, target) in [
        ("open", "open"),
        ("complete", "completed"),
        ("lock", "locked"),
    ] {
        if action == "complete" {
            ready_to_complete(&pool, &f).await;
        }
        let before = stored(&pool).await;
        if action == "open" {
            assert_eq!(before["handicap_locks"], serde_json::json!([]));
            assert_eq!(before["players"], serde_json::json!([]));
            assert_eq!(before["teams"], serde_json::json!([]));
        }
        let (session, token) = new_session(&pool, true).await;
        let (wait_gate, pid) = gate(&pool, late).await;
        let app = f.app.clone();
        let mut task = tokio::spawn(async move { request(app, action, &token).await });
        tokio::select! {
            _ = wait_blocked(&pool, pid) => {},
            response = &mut task => panic!("{format} {action} late={late} returned before wait: {}", json(response.unwrap()).await),
        }
        assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
        wait_expired(&pool, session).await;
        wait_gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{format} {action} late={late}"
        );
        assert!(
            response.headers()[CACHE_CONTROL]
                .to_str()
                .unwrap()
                .contains("no-store")
        );
        assert_eq!(json(response).await["error"]["code"], "unauthenticated");
        assert_eq!(
            stored(&pool).await,
            before,
            "all lifecycle effects must roll back"
        );
        assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

        // A renewed valid session waits at the same stage and commits once.
        let (_, token) = new_session(&pool, false).await;
        let (wait_gate, pid) = gate(&pool, late).await;
        let app = f.app.clone();
        let mut task = tokio::spawn(async move { request(app, action, &token).await });
        tokio::select! {
            _ = wait_blocked(&pool, pid) => {},
            response = &mut task => panic!("valid {format} {action} late={late} returned before wait: {}", json(response.unwrap()).await),
        }
        assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
        wait_gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "valid {format} {action}");
        let result = json(response).await;
        assert_eq!(
            if action == "open" {
                &result["round"]["status"]
            } else {
                &result["status"]
            },
            target
        );
        let after = stored(&pool).await;
        assert_eq!(after["round"]["status"], target);
        let event = events.try_recv().unwrap();
        assert_eq!(
            (event.resource, event.tournament_id, event.id),
            ("round", id(2), id(5))
        );
        assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
        if action == "open" {
            assert_eq!(after["players"].as_array().unwrap().len(), 4);
            assert_eq!(after["handicap_locks"].as_array().unwrap().len(), 1);
            assert_eq!(
                after["teams"].as_array().unwrap().len(),
                if format == "two_player_foursomes" {
                    2
                } else {
                    0
                }
            );
        } else {
            for key in [
                "players",
                "teams",
                "handicap_locks",
                "scores",
                "score_audits",
                "confirmations",
                "matches",
                "opponents",
                "match_notes",
                "match_audits",
                "match_receipts",
            ] {
                assert_eq!(
                    after[key], before[key],
                    "valid transition must preserve {key}"
                );
            }
        }
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn individual_membership_expiry_rolls_back_all_actions(pool: PgPool) {
    cycle(pool, "individual_stroke_play", false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn individual_late_expiry_rolls_back_all_actions(pool: PgPool) {
    cycle(pool, "individual_stroke_play", true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn foursomes_membership_expiry_rolls_back_all_actions(pool: PgPool) {
    cycle(pool, "two_player_foursomes", false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn foursomes_late_expiry_rolls_back_all_actions(pool: PgPool) {
    cycle(pool, "two_player_foursomes", true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn match_membership_expiry_rolls_back_all_actions(pool: PgPool) {
    cycle(pool, "singles_match_play", false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn match_late_expiry_rolls_back_all_actions(pool: PgPool) {
    cycle(pool, "singles_match_play", true).await;
}
