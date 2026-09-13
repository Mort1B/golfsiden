use chrono::{Duration, Utc};
use golf_api::{
    auth::hash_session_token,
    domain::{
        scorecards::{ExpectedScore, ScoreOwner},
        stableford::card::Input,
    },
    repositories::{
        auth, round_lifecycle,
        stableford::{self, SaveInput},
        tournaments,
    },
};
use sqlx::PgPool;
use uuid::Uuid;
pub fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
pub const TOKEN: &str = "stableford-admin-test-token";
pub async fn draft(pool: &PgPool, format: &str) -> Uuid {
    draft_rounds(pool, format, 1).await
}
pub async fn draft_rounds(pool: &PgPool, format: &str, count: i16) -> Uuid {
    sqlx::query(
        "INSERT INTO users(id,username,display_name,role) VALUES($1,'fb_admin','Admin','admin')",
    )
    .bind(id(1))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES($1,'Four-ball cup','2026-09-13','2026-09-13',$2,$3)").bind(id(2)).bind(count).bind(if format == "singles_match_play" { None } else { Some(1i16) }).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(id(2))
    .bind(id(1))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO courses(id,name) VALUES($1,'Links')")
        .bind(id(3))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tees(id,course_id,name,slope_rating,course_rating) VALUES($1,$2,'White',113,72)").bind(id(4)).bind(id(3)).execute(pool).await.unwrap();
    for hole in 1..=18 {
        sqlx::query(
            "INSERT INTO holes(id,tee_id,hole_number,par,stroke_index) VALUES($1,$2,$3,4,$3)",
        )
        .bind(id(100 + hole))
        .bind(id(4))
        .bind(hole as i16)
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_id,course_name,tee_id,tee_name,number_of_holes,scoring_format,handicap_allowance_percent) VALUES($1,$2,1,'Individual','2026-09-13',$3,'Links',$4,'White',18,$5::text::scoring_format,100)").bind(id(5)).bind(id(2)).bind(id(3)).bind(id(4)).bind(format).execute(pool).await.unwrap();
    if count >= 2 {
        sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,number_of_holes,scoring_format,handicap_allowance_percent) VALUES($1,$2,2,'Draft Stableford','2026-09-13','Unconfigured','Unconfigured',18,'individual_stableford',100)")
            .bind(id(6)).bind(id(2)).execute(pool).await.unwrap();
    }
    if count == 3 {
        sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_id,course_name,tee_id,tee_name,number_of_holes,scoring_format,handicap_allowance_percent) VALUES($1,$2,3,'Final Match','2026-09-13',$3,'Links',$4,'White',18,'singles_match_play',100)").bind(id(7)).bind(id(2)).bind(id(3)).bind(id(4)).execute(pool).await.unwrap();
    }
    for (p, hcp) in [(11, 0), (12, 40), (13, -10), (14, 10)] {
        sqlx::query("INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,$2,$3)")
            .bind(id(p))
            .bind(format!("Player {p}"))
            .bind(hcp)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES($1,$2,$3)").bind(id(2)).bind(id(p)).bind(hcp).execute(pool).await.unwrap();
    }
    for (_team, flight, players) in [(21, 31, [11, 12]), (22, 32, [13, 14])] {
        sqlx::query("INSERT INTO flights(id,round_id,tournament_id,name) VALUES($1,$2,$3,$4)")
            .bind(id(flight))
            .bind(id(5))
            .bind(id(2))
            .bind(format!("Flight {flight}"))
            .execute(pool)
            .await
            .unwrap();
        for player in players {
            sqlx::query("INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(id(flight)).bind(id(5)).bind(id(2)).bind(id(player)).execute(pool).await.unwrap();
        }
    }
    let session = auth::create_session(
        pool,
        id(1),
        &hash_session_token(TOKEN),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    let updated = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(id(2))
        .fetch_one(pool)
        .await
        .unwrap();
    let updated = if count == 3 {
        tournaments::update_counted_rounds_authorized(
            pool,
            session.session_id,
            id(2),
            1,
            None,
            Some(golf_api::domain::models::TournamentTieBreakPolicy::FinalRoundScore),
            updated,
        )
        .await
        .unwrap()
        .tournament
        .updated_at
    } else {
        updated
    };
    tournaments::start_authorized(pool, session.session_id, id(2), updated)
        .await
        .unwrap();
    session.session_id
}
pub async fn ready(pool: &PgPool) -> Uuid {
    let session = draft(pool, "individual_stableford").await;
    round_lifecycle::open(pool, id(5)).await.unwrap();
    session
}
pub fn operation(
    session: Uuid,
    player: u128,
    hole: u128,
    input: Input,
    expected_score: ExpectedScore,
) -> SaveInput {
    SaveInput {
        request_id: Uuid::new_v4(),
        round_id: id(5),
        hole_id: id(100 + hole),
        owner: ScoreOwner::Player { id: id(player) },
        input,
        expected_score,
        session_id: session,
    }
}
pub async fn numeric(pool: &PgPool, session: Uuid, player: u128, hole: u128, gross: i16) {
    stableford::save_conditional(
        pool,
        operation(
            session,
            player,
            hole,
            Input::Numeric {
                gross_strokes: gross,
            },
            ExpectedScore::Absent {},
        ),
    )
    .await
    .unwrap();
}
pub async fn fill(pool: &PgPool, session: Uuid) {
    for hole in 1..=18 {
        for player in [11, 12, 13, 14] {
            numeric(pool, session, player, hole, 4).await;
        }
    }
}
pub async fn session(
    pool: &PgPool,
    user: u128,
    player: Option<u128>,
    role: &str,
    token: &str,
) -> Uuid {
    sqlx::query("INSERT INTO users(id,username,display_name,role,player_id) VALUES($1,$2,'Member','player',$3)").bind(id(user)).bind(format!("fb_user_{user}")).bind(player.map(id)).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,$3::text::tournament_role)").bind(id(2)).bind(id(user)).bind(role).execute(pool).await.unwrap();
    auth::create_session(
        pool,
        id(user),
        &hash_session_token(token),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id
}
