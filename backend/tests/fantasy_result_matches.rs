#![cfg(feature = "database-tests")]
mod fantasy_results_support;
use fantasy_results_support::*;
use golf_api::domain::match_play::commands::{
    Basis, Command, CorrectionKind, Event, Outcome, Request as MatchRequest,
};
async fn setup(pool: &PgPool, f: &Fantasy, r: Uuid) -> Uuid {
    sqlx::query(
        "UPDATE rounds SET scoring_format='singles_match_play',handicap_enabled=false WHERE id=$1",
    )
    .bind(r)
    .execute(pool)
    .await
    .unwrap();
    let id = Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('app.match_actor',$1,true),set_config('app.match_session',$2,true)",
    )
    .bind(f.base.admin.to_string())
    .bind(f.base.session.to_string())
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("INSERT INTO singles_matches(id,round_id,tournament_id,first_player_id,second_player_id) VALUES($1,$2,$3,$4,$5)").bind(id).bind(r).bind(f.base.tournament).bind(f.players[0]).bind(f.players[1]).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    fantasy::save(pool, f.base.session, f.base.tournament, r, &lineup(f))
        .await
        .unwrap();
    close(pool, f, r).await;
    source_fixture(pool, f, r).await;
    id
}
async fn command(pool: &PgPool, f: &Fantasy, r: Uuid, id: Uuid, revision: i64, command: Command) {
    match_play::execute(
        pool,
        f.base.session,
        r,
        id,
        MatchRequest {
            request_id: Uuid::new_v4(),
            expected_revision: ScoreRevision::from_database(revision).unwrap(),
            command,
        },
    )
    .await
    .unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn match_results_override_dnf_ignore_notes_and_double_negative_captain(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let id = setup(&pool, &f, r).await;
    for p in &f.players {
        disposition(&pool, &f, r, OwnerKind::Player, *p, false).await;
    }
    let v = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&v, f.players[0])["points"],
        json!({"state":"settled","total":0})
    );
    command(
        &pool,
        &f,
        r,
        id,
        1,
        Command::Note {
            player_id: f.players[0],
            hole_number: 1,
            gross_strokes: 1,
        },
    )
    .await;
    let v = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&v, f.players[0])["points"],
        json!({"state":"pending","recorded":0})
    );
    command(
        &pool,
        &f,
        r,
        id,
        2,
        Command::Report {
            event: Event::Concession {
                conceding_player_id: f.players[1],
                communicated: true,
                after_hole: 0,
            },
        },
    )
    .await;
    let v = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&v, f.players[0])["points"],
        json!({"state":"provisional","total":3})
    );
    assert_eq!(
        golfer(&v, f.players[1])["points"],
        json!({"state":"provisional","total":-1})
    );
    assert_eq!(
        manager(&v, f.base.admin)["points"],
        json!({"state":"provisional","total":1})
    );
    command(
        &pool,
        &f,
        r,
        id,
        3,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    let v = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        manager(&v, f.base.admin)["points"],
        json!({"state":"settled","total":1})
    );
    let event: Uuid =
        sqlx::query_scalar("SELECT (ledger->0->>'id')::uuid FROM singles_matches WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    command(
        &pool,
        &f,
        r,
        id,
        4,
        Command::Correct {
            kind: CorrectionKind::RecordingError,
            reason: "Correct winner".into(),
            superseded_event_ids: vec![event],
            replacement: vec![Event::Concession {
                conceding_player_id: f.players[0],
                communicated: true,
                after_hole: 0,
            }],
        },
    )
    .await;
    let v = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(golfer(&v, f.players[0])["match_outcome"], "loss");
    assert_eq!(manager(&v, f.base.admin)["points"]["total"], 5);
}
#[sqlx::test(migrations = "../migrations")]
async fn hidden_match_outcomes_confirmations_and_corrections_do_not_change_member_json(
    pool: PgPool,
) {
    let f = fixture(&pool).await;
    let r = f.rounds[2];
    let id = setup(&pool, &f, r).await;
    hide(&pool, &f, true).await;
    let before = scores(&pool, &f, r, f.member_session).await;
    let total = serde_json::to_value(
        results::overall(&pool, f.member_session, f.base.tournament)
            .await
            .unwrap(),
    )
    .unwrap();
    command(
        &pool,
        &f,
        r,
        id,
        1,
        Command::Report {
            event: Event::Concession {
                conceding_player_id: f.players[1],
                communicated: true,
                after_hole: 0,
            },
        },
    )
    .await;
    command(
        &pool,
        &f,
        r,
        id,
        2,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    assert_eq!(before, scores(&pool, &f, r, f.member_session).await);
    assert_eq!(
        total,
        serde_json::to_value(
            results::overall(&pool, f.member_session, f.base.tournament)
                .await
                .unwrap()
        )
        .unwrap()
    );
    let event: Uuid =
        sqlx::query_scalar("SELECT (ledger->0->>'id')::uuid FROM singles_matches WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    command(
        &pool,
        &f,
        r,
        id,
        3,
        Command::Correct {
            kind: CorrectionKind::RecordingError,
            reason: "Correct winner".into(),
            superseded_event_ids: vec![event],
            replacement: vec![Event::Concession {
                conceding_player_id: f.players[0],
                communicated: true,
                after_hole: 0,
            }],
        },
    )
    .await;
    assert_eq!(before, scores(&pool, &f, r, f.member_session).await);
    assert_ne!(before, scores(&pool, &f, r, f.base.session).await);
}
#[sqlx::test(migrations = "../migrations")]
async fn completed_draw_awards_one_each_without_hole_bonuses(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let id = setup(&pool, &f, r).await;
    for h in 1..=18 {
        command(
            &pool,
            &f,
            r,
            id,
            i64::from(h),
            Command::Report {
                event: Event::Hole {
                    hole_number: h,
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
        &pool,
        &f,
        r,
        id,
        19,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    let v = scores(&pool, &f, r, f.base.session).await;
    for p in &f.players[..2] {
        assert_eq!(
            golfer(&v, *p)["points"],
            json!({"state":"settled","total":1})
        );
        assert_eq!(golfer(&v, *p)["match_outcome"], "draw");
    }
}
