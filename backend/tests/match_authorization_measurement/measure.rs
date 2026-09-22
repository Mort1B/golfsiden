use crate::fixture::{self, id};
use golf_api::{
    domain::{
        match_play::commands::{Command, Event, Request},
        scorecards::ScoreRevision,
    },
    repositories::{
        match_play::{
            self,
            reads::Listing,
            setup::{Assignments, Pair},
        },
        round_completion, round_lifecycle, tournament_visibility,
    },
};
use serde_json::{Value, json};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use std::time::Instant;
use uuid::Uuid;

async fn listing(pool: &PgPool, session: Uuid, filtered: bool, target: Uuid) -> Listing {
    if filtered {
        match_play::reads::list_for_player(pool, session, id(5), target)
            .await
            .unwrap()
    } else {
        match_play::reads::list(pool, session, id(5)).await.unwrap()
    }
}
fn parity(full: &Value, selected: &Value, target: Uuid) {
    let cards: Vec<_> = full["matches"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| {
            c["opponents"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["player_id"] == target.to_string())
        })
        .cloned()
        .collect();
    let writable: Vec<_> = full["writable_match_ids"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| cards.iter().any(|c| c["match_id"] == **i))
        .cloned()
        .collect();
    assert_eq!(
        *selected,
        json!({"round_id":id(5),"player_id":target,"matches":cards,"writable_match_ids":writable})
    );
}
async fn stats(pool: &PgPool) -> Vec<Value> {
    sqlx::query("SELECT query,calls,rows,total_exec_time,shared_blks_hit,shared_blks_read FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND toplevel AND query NOT LIKE '%pg_stat_statements%' ORDER BY query")
        .fetch_all(pool).await.unwrap().into_iter().map(|r| json!({
            "query":r.get::<String,_>("query"), "calls":r.get::<i64,_>("calls"), "rows":r.get::<i64,_>("rows"),
            "execution_ms":r.get::<f64,_>("total_exec_time"), "shared_hits":r.get::<i64,_>("shared_blks_hit"), "shared_reads":r.get::<i64,_>("shared_blks_read")
        })).collect()
}
pub async fn run(pool: PgPool, matches: u128) {
    assert_eq!(
        std::env::var("GOLF_AUTH_MEASURE").as_deref(),
        Ok("1"),
        "explicit isolated measurement opt-in required"
    );
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_stat_statements")
        .execute(&pool)
        .await
        .unwrap();
    let admin = fixture::setup(&pool, matches).await;
    let scorer = fixture::session(&pool, 201, None, "scorer", "synthetic-measurement-scorer").await;
    let player = fixture::session(
        &pool,
        202,
        Some(1000),
        "player",
        "synthetic-measurement-player",
    )
    .await;
    let viewer = fixture::session(&pool, 203, None, "viewer", "synthetic-measurement-viewer").await;
    let updated = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
        .bind(id(5))
        .fetch_one(&pool)
        .await
        .unwrap();
    match_play::setup::replace(
        &pool,
        admin,
        id(5),
        Assignments {
            expected_round_updated_at: updated,
            matches: (0..matches)
                .map(|i| Pair {
                    first_player_id: id(1000 + 2 * i),
                    second_player_id: id(1001 + 2 * i),
                })
                .collect(),
        },
    )
    .await
    .unwrap();
    round_lifecycle::open(&pool, id(5)).await.unwrap();
    let cards = listing(&pool, admin, false, id(1000)).await;
    for card in cards.matches {
        for (revision, command) in [
            (
                1,
                Command::Note {
                    player_id: card.opponents[0].player_id,
                    hole_number: 1,
                    gross_strokes: 4,
                },
            ),
            (
                2,
                Command::Note {
                    player_id: card.opponents[1].player_id,
                    hole_number: 10,
                    gross_strokes: 5,
                },
            ),
            (
                3,
                Command::Report {
                    event: Event::Concession {
                        conceding_player_id: card.opponents[1].player_id,
                        communicated: true,
                        after_hole: 0,
                    },
                },
            ),
            (
                4,
                Command::Confirm {
                    result_agreed_or_awarded: true,
                },
            ),
        ] {
            match_play::execute(
                &pool,
                admin,
                id(5),
                card.match_id,
                Request {
                    request_id: Uuid::new_v4(),
                    expected_revision: ScoreRevision::from_database(revision).unwrap(),
                    command,
                },
            )
            .await
            .unwrap();
        }
    }
    // Use one established connection for repeated samples; a separately opened
    // connection measures its first repository call without including connect time.
    let options = pool.connect_options();
    let measured = PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*options).clone())
        .await
        .unwrap();
    let mut evidence = Vec::new();
    for state in if matches == 24 {
        vec!["open", "locked"]
    } else {
        vec!["open"]
    } {
        if state == "locked" {
            round_completion::complete_authorized(&pool, admin, id(5))
                .await
                .unwrap();
            round_completion::lock_authorized(&pool, admin, id(5))
                .await
                .unwrap();
        }
        for hidden in [true, false] {
            let v = tournament_visibility::get_for_admin(&pool, id(1), id(2))
                .await
                .unwrap();
            tournament_visibility::update_authorized(
                &pool,
                admin,
                id(2),
                hidden,
                v.visibility_updated_at,
            )
            .await
            .unwrap();
            for (role, session) in [
                ("admin", admin),
                ("scorer", scorer),
                ("player", player),
                ("viewer", viewer),
                ("player_other", player),
            ] {
                let target = if role == "player_other" {
                    id(1002)
                } else {
                    id(1000)
                };
                let full =
                    serde_json::to_value(listing(&measured, session, false, target).await).unwrap();
                let selected =
                    serde_json::to_value(listing(&measured, session, true, target).await).unwrap();
                parity(&full, &selected, target);
                assert_eq!(full["matches"].as_array().unwrap().len(), matches as usize);
                assert_eq!(selected["matches"].as_array().unwrap().len(), 1);
                let expected_writable = if role == "viewer" || state == "locked" && role != "admin"
                {
                    0
                } else if role.starts_with("player") {
                    1
                } else {
                    matches as usize
                };
                assert_eq!(
                    full["writable_match_ids"].as_array().unwrap().len(),
                    expected_writable
                );
                assert_eq!(
                    selected["matches"][0]["confirmed"],
                    if hidden && role != "admin" {
                        Value::Null
                    } else {
                        json!(true)
                    }
                );
                // Rotate list order by role, reducing a fixed order bias.
                for filtered in if role == "player_other" {
                    vec![true]
                } else if role == "scorer" || role == "viewer" {
                    vec![true, false]
                } else {
                    vec![false, true]
                } {
                    let expected = if filtered { &selected } else { &full };
                    let fresh = PgPoolOptions::new()
                        .max_connections(1)
                        .connect_with((*options).clone())
                        .await
                        .unwrap();
                    let start = Instant::now();
                    let first = listing(&fresh, session, filtered, target).await;
                    let first_ms = start.elapsed().as_secs_f64() * 1000.0;
                    assert_eq!(serde_json::to_value(first).unwrap(), *expected);
                    fresh.close().await;
                    for _ in 0..3 {
                        listing(&measured, session, filtered, target).await;
                    }
                    sqlx::query("SELECT pg_stat_statements_reset()")
                        .execute(&pool)
                        .await
                        .unwrap();
                    let mut samples = Vec::new();
                    for _ in 0..10 {
                        let start = Instant::now();
                        let result = listing(&measured, session, filtered, target).await;
                        samples.push(start.elapsed().as_secs_f64() * 1000.0);
                        assert_eq!(serde_json::to_value(result).unwrap(), *expected);
                    }
                    let queries = stats(&pool).await;
                    assert!(!queries.is_empty());
                    evidence.push(json!({"matches":matches as u64,"players":matches as u64*2,"state":state,"hidden":hidden,"role":role,"filtered":filtered,"fresh_connection_first_ms":first_ms,"warm_ms":samples,"queries":queries,"semantic_parity":true,"writable":expected["writable_match_ids"].as_array().unwrap().len()}));
                }
            }
        }
    }
    // The same live session must lose both list variants after membership removal.
    sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
        .bind(id(202))
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        match_play::reads::list(&pool, player, id(5)).await,
        Err(match_play::Error::Forbidden)
    ));
    assert!(matches!(
        match_play::reads::list_for_player(&pool, player, id(5), id(1000)).await,
        Err(match_play::Error::Forbidden)
    ));
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&pool)
        .await
        .unwrap();
    let settings: Vec<Value> = sqlx::query("SELECT name,setting,unit FROM pg_settings WHERE name IN ('shared_buffers','work_mem','max_connections','jit','track_io_timing','pg_stat_statements.track','pg_stat_statements.track_planning') ORDER BY name").fetch_all(&pool).await.unwrap().into_iter().map(|r| json!({"name":r.get::<String,_>("name"),"setting":r.get::<String,_>("setting"),"unit":r.get::<Option<String>,_>("unit")})).collect();
    let out = std::env::var("GOLF_AUTH_MEASURE_OUT").expect("output directory required");
    std::fs::write(
        std::path::Path::new(&out).join(format!("{matches}.json")),
        serde_json::to_string_pretty(
            &json!({"postgres":version,"settings":settings,"samples":evidence,"membership_removal_denied":true}),
        )
        .unwrap(),
    )
    .unwrap();
    measured.close().await;
}
