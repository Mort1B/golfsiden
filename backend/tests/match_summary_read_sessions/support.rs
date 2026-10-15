pub use crate::card_support::{
    PLAYER, ROUND, TRIP, USER, app, assert_private, blocker, body, expire_at_wait, fixture, get,
    lock_membership, match_id, score_state, session, submit, wait_blocked,
};
pub fn paths() -> Vec<String> {
    vec![
        format!("/api/rounds/{ROUND}/match-play/completion"),
        format!("/api/tournaments/{TRIP}/match-table"),
    ]
}

pub async fn empty_fixture(pool: &sqlx::PgPool) -> std::sync::Arc<golf_api::AppState> {
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES($1,'summary_admin','Admin','admin')").bind(USER).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES($1,'Empty match cup','2026-10-15','2026-10-15',1,NULL)").bind(TRIP).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(TRIP)
    .bind(USER)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,number_of_holes,scoring_format,handicap_allowance_percent) VALUES($1,$2,1,'Empty match round','2026-10-15','Unconfigured','Unconfigured',18,'singles_match_play',100)").bind(ROUND).bind(TRIP).execute(pool).await.unwrap();
    golf_api::AppState::new(pool.clone())
}
pub async fn command(
    pool: &sqlx::PgPool,
    session: uuid::Uuid,
    target: uuid::Uuid,
    revision: i64,
    command: golf_api::domain::match_play::commands::Command,
) {
    golf_api::repositories::match_play::execute(
        pool,
        session,
        ROUND,
        target,
        golf_api::domain::match_play::commands::Request {
            request_id: uuid::Uuid::new_v4(),
            expected_revision: golf_api::domain::scorecards::ScoreRevision::from_database(revision)
                .unwrap(),
            command,
        },
    )
    .await
    .unwrap();
}
pub async fn finish_second_draw(pool: &sqlx::PgPool, session: uuid::Uuid) {
    use golf_api::domain::match_play::commands::{Basis, Command, Event, Outcome};
    let target = sqlx::query_scalar(
        "SELECT id FROM singles_matches WHERE round_id=$1 AND first_player_id=$2",
    )
    .bind(ROUND)
    .bind(uuid::Uuid::from_u128(13))
    .fetch_one(pool)
    .await
    .unwrap();
    for hole in 1..=18 {
        command(
            pool,
            session,
            target,
            i64::from(hole),
            Command::Report {
                event: Event::Hole {
                    hole_number: hole,
                    outcome: Outcome::Halved,
                    basis: Basis::AgreedHalve {
                        play_begun: true,
                        mutual_agreement: true,
                    },
                },
            },
        )
        .await;
    }
    command(
        pool,
        session,
        target,
        19,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
}
pub async fn populate(pool: &sqlx::PgPool, session: uuid::Uuid) {
    use golf_api::domain::match_play::commands::{Command, Event};
    submit(
        pool,
        session,
        1,
        Command::Report {
            event: Event::Concession {
                conceding_player_id: uuid::Uuid::from_u128(12),
                communicated: true,
                after_hole: 0,
            },
        },
    )
    .await;
    submit(
        pool,
        session,
        2,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    finish_second_draw(pool, session).await;
}
