#![cfg(feature = "database-tests")]
mod singles_match {
    mod api;
    mod commands;
    mod concurrency;
    mod configuration;
    #[path = "../stableford/support.rs"]
    #[allow(dead_code)]
    mod fixture;
    mod guards;
    mod listing;
    mod listing_api;
    mod overall;
    mod privacy;
    mod setup;
    mod upgrade;
    use fixture::*;
    use golf_api::{
        domain::{
            match_play::commands::{Acknowledgement, Command, Request},
            scorecards::ScoreRevision,
        },
        repositories::{
            match_play::{
                self,
                setup::{Assignments, Pair},
            },
            round_lifecycle,
        },
    };
    use sqlx::PgPool;
    use uuid::Uuid;
    async fn ready(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
        let session = fixture::draft(pool, "singles_match_play").await;
        let updated = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
            .bind(id(5))
            .fetch_one(pool)
            .await
            .unwrap();
        match_play::setup::replace(
            pool,
            session,
            id(5),
            Assignments {
                expected_round_updated_at: updated,
                matches: vec![
                    Pair {
                        first_player_id: id(11),
                        second_player_id: id(12),
                    },
                    Pair {
                        first_player_id: id(13),
                        second_player_id: id(14),
                    },
                ],
            },
        )
        .await
        .unwrap();
        round_lifecycle::open(pool, id(5)).await.unwrap();
        let ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM singles_matches WHERE round_id=$1 ORDER BY first_player_id",
        )
        .bind(id(5))
        .fetch_all(pool)
        .await
        .unwrap();
        (session, ids[0], ids[1])
    }
    fn request(revision: i64, command: Command) -> Request {
        Request {
            request_id: Uuid::new_v4(),
            expected_revision: ScoreRevision::from_database(revision).unwrap(),
            command,
        }
    }
    async fn submit(
        pool: &PgPool,
        session: Uuid,
        match_id: Uuid,
        revision: i64,
        command: Command,
    ) -> Acknowledgement {
        match_play::execute(pool, session, id(5), match_id, request(revision, command))
            .await
            .unwrap()
            .value
    }
}
