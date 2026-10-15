use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::hash_session_token,
    domain::models::{Team, TeamMember, TeamWithMembers},
    repositories::auth,
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub fn id(n: u128) -> Uuid {
    Uuid::from_u128(0xa8000000000000000000000000000000 + n)
}

pub async fn setup(pool: &PgPool) -> (Router, Uuid, String) {
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES($1,'team_probe','Synthetic reader','viewer')").bind(id(1)).execute(pool).await.unwrap();
    for trip in [id(2), id(3)] {
        sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Synthetic teams','2026-10-01','2026-10-05',5)").bind(trip).execute(pool).await.unwrap();
    }
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'viewer')",
    )
    .bind(id(2))
    .bind(id(1))
    .execute(pool)
    .await
    .unwrap();
    let token = Uuid::new_v4().to_string();
    let principal = auth::create_session(
        pool,
        id(1),
        &hash_session_token(&token),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    (
        api::router(Arc::clone(&AppState::new(pool.clone()))),
        principal.session_id,
        token,
    )
}

pub async fn round(pool: &PgPool, number: i16, count: u128, other_trip: bool) -> Uuid {
    let trip = id(if other_trip { 3 } else { 2 });
    let round = id(100 + number as u128);
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES($1,$2,$3,'Synthetic round','2026-10-01','','','team_scramble')").bind(round).bind(trip).bind(number).execute(pool).await.unwrap();
    for index in 0..count {
        let team = id(1000 + number as u128 * 1000 + index);
        // Unique names with a different order from UUIDs; SQL chooses native collation.
        let name = format!(
            "{}-{:03}",
            if index % 2 == 0 { "b-team" } else { "B-team" },
            count - index
        );
        sqlx::query("INSERT INTO teams(id,round_id,tournament_id,name,starting_hole,tee_time,created_at,updated_at) VALUES($1,$2,$3,$4,$5,$6,'2026-10-01T00:00:00Z','2026-10-01T00:00:00Z')")
            .bind(team).bind(round).bind(trip).bind(name)
            .bind(if index % 2 == 0 { Some(3i16) } else { None })
            .bind(if index % 2 == 0 { chrono::NaiveTime::from_hms_opt(8,30,0) } else { None })
            .execute(pool).await.unwrap();
        if index % 4 == 3 {
            continue;
        } // Empty teams are retained.
        for (member, (name, order)) in [
            ("z-last", Some(2i16)),
            ("B-order", Some(1)),
            ("b-order", Some(1)),
            ("b-order", Some(1)),
            ("B-null", None),
            ("b-null", None),
            ("b-null", None),
            ("a-null", None),
        ]
        .into_iter()
        .enumerate()
        {
            let player = id(100_000 + number as u128 * 100_000 + index * 10 + member as u128);
            sqlx::query(
                "INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,$2,12)",
            )
            .bind(player)
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
            sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES($1,$2,12)").bind(trip).bind(player).execute(pool).await.unwrap();
            sqlx::query("INSERT INTO team_memberships(team_id,round_id,tournament_id,player_id,display_order) VALUES($1,$2,$3,$4,$5)").bind(team).bind(round).bind(trip).bind(player).bind(order).execute(pool).await.unwrap();
        }
    }
    round
}

// The original ordered N+1 read is an independent response oracle, never timed.
pub async fn original(pool: &PgPool, round: Uuid) -> Value {
    let mut tx = pool.begin().await.unwrap();
    let teams = sqlx::query_as::<_, Team>("SELECT id,round_id,tournament_id,name,starting_hole,tee_time,created_at,updated_at FROM teams WHERE round_id=$1 ORDER BY name,id").bind(round).fetch_all(&mut *tx).await.unwrap();
    let mut result = Vec::new();
    for team in teams {
        let members = sqlx::query_as::<_, TeamMember>("SELECT tm.player_id,p.display_name,tm.display_order FROM team_memberships tm JOIN players p ON p.id=tm.player_id WHERE tm.team_id=$1 ORDER BY tm.display_order NULLS LAST,p.display_name,tm.player_id").bind(team.id).fetch_all(&mut *tx).await.unwrap();
        result.push(TeamWithMembers { team, members });
    }
    tx.commit().await.unwrap();
    serde_json::to_value(result).unwrap()
}

pub async fn response(app: &Router, round: Uuid, token: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::get(format!("/api/rounds/{round}/teams"))
                .header(header::COOKIE, format!("golf_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

pub async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

pub async fn http(app: &Router, round: Uuid, token: &str) -> Value {
    let r = response(app, round, token).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
    body(r).await
}
