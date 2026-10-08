#![allow(dead_code, unused_imports)]
#[path = "../player_claims_support/mod.rs"]
mod base;
pub use base::*;
pub use golf_api::repositories::fantasy::{self, Dispose, Error, OwnerKind, Save};
pub struct Fantasy {
    pub base: Fixture,
    pub players: Vec<Uuid>,
    pub rounds: Vec<Uuid>,
    pub member_session: Uuid,
}
pub async fn fixture(pool: &PgPool) -> Fantasy {
    let base = seed(pool).await;
    sqlx::query("UPDATE tournaments SET number_of_rounds=3 WHERE id=$1")
        .bind(base.tournament)
        .execute(pool)
        .await
        .unwrap();
    let mut players = vec![];
    for _ in 0..5 {
        players.push(prepared(pool, &base).await.0.player_id);
    }
    let mut rounds = vec![];
    for number in 1..=3i16 {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES($1,$2,$3,'Fantasy Round','2026-10-01','','','individual_stroke_play')").bind(id).bind(base.tournament).bind(number).execute(pool).await.unwrap();
        rounds.push(id);
    }
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'viewer')",
    )
    .bind(base.tournament)
    .bind(base.other)
    .execute(pool)
    .await
    .unwrap();
    let member_session = auth::create_session(
        pool,
        base.other,
        &hash_session_token("fantasy-member"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id;
    fantasy::configure(pool, base.session, base.tournament, true)
        .await
        .unwrap();
    fantasy::enter(pool, base.session, base.tournament)
        .await
        .unwrap();
    fantasy::enter(pool, member_session, base.tournament)
        .await
        .unwrap();
    Fantasy {
        base,
        players,
        rounds,
        member_session,
    }
}
pub fn lineup(f: &Fantasy) -> Save {
    Save {
        request_id: Uuid::new_v4(),
        expected_revision: 0,
        picks: f.players[..4].to_vec(),
        captain: f.players[1],
    }
}
pub async fn close(pool: &PgPool, f: &Fantasy, r: Uuid) {
    fantasy::set_deadline(
        pool,
        f.base.session,
        f.base.tournament,
        r,
        Some(Utc::now() + Duration::milliseconds(100)),
    )
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
}
pub async fn read(pool: &PgPool, f: &Fantasy, r: Uuid) -> fantasy::RoundView {
    fantasy::read_round(pool, f.base.session, f.base.tournament, r)
        .await
        .unwrap()
}
pub async fn course_fixture(pool: &PgPool, _f: &Fantasy, r: Uuid) {
    let course = Uuid::new_v4();
    let tee = Uuid::new_v4();
    sqlx::query("INSERT INTO courses(id,name) VALUES($1,'Fantasy Course')")
        .bind(course)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tees(id,course_id,name,slope_rating,course_rating) VALUES($1,$2,'White',113,72)").bind(tee).bind(course).execute(pool).await.unwrap();
    for n in 1..=18i16 {
        sqlx::query(
            "INSERT INTO holes(id,tee_id,hole_number,par,stroke_index) VALUES($1,$2,$3,4,$3)",
        )
        .bind(Uuid::new_v4())
        .bind(tee)
        .bind(n)
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE rounds SET course_id=$2,tee_id=$3 WHERE id=$1")
        .bind(r)
        .bind(course)
        .bind(tee)
        .execute(pool)
        .await
        .unwrap();
}

pub async fn source_fixture(pool: &PgPool, f: &Fantasy, r: Uuid) {
    course_fixture(pool, f, r).await;
    for p in &f.players {
        historical_status(pool, r, *p, "open").await;
    }
}
pub async fn ready_round(pool: &PgPool, f: &Fantasy, r: Uuid) {
    course_fixture(pool, f, r).await;
    for (index, group) in f.players.chunks(3).enumerate() {
        let flight = Uuid::new_v4();
        sqlx::query("INSERT INTO flights(id,round_id,tournament_id,name) VALUES($1,$2,$3,$4)")
            .bind(flight)
            .bind(r)
            .bind(f.base.tournament)
            .bind(format!("Flight {index}"))
            .execute(pool)
            .await
            .unwrap();
        for p in group {
            sqlx::query("INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(flight).bind(r).bind(f.base.tournament).bind(p).execute(pool).await.unwrap();
        }
    }
    let updated = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(f.base.tournament)
        .fetch_one(pool)
        .await
        .unwrap();
    golf_api::repositories::tournaments::start_authorized(
        pool,
        f.base.session,
        f.base.tournament,
        updated,
    )
    .await
    .unwrap();
}
