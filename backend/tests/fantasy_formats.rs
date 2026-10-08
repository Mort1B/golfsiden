#![cfg(feature = "database-tests")]
mod fantasy_support;
use fantasy_support::*;
use golf_api::{
    domain::{
        four_ball_card::Input,
        match_play::commands::{Command, CorrectionKind, Event, Request as MatchRequest},
        scorecards::{ExpectedScore, ScoreOwner, ScoreRevision},
    },
    repositories::{four_ball, match_play, stableford},
};
async fn source(pool: &PgPool, f: &Fantasy, r: Uuid, kind: OwnerKind, p: Uuid) -> String {
    fantasy::source(pool, f.base.session, f.base.tournament, r, kind, p)
        .await
        .unwrap()
        .source_token
}
async fn hole(pool: &PgPool, r: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT h.id FROM holes h JOIN rounds r ON r.tee_id=h.tee_id WHERE r.id=$1 ORDER BY hole_number LIMIT 1").bind(r).fetch_one(pool).await.unwrap()
}
#[sqlx::test(migrations = "../migrations")]
async fn stableford_pickup_changes_token_but_exact_replay_does_not(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let p = f.players[0];
    sqlx::query("UPDATE rounds SET scoring_format='individual_stableford' WHERE id=$1")
        .bind(r)
        .execute(&pool)
        .await
        .unwrap();
    source_fixture(&pool, &f, r).await;
    let before = source(&pool, &f, r, OwnerKind::Player, p).await;
    let h = hole(&pool, r).await;
    let request = Uuid::new_v4();
    for _ in 0..2 {
        stableford::save_conditional(
            &pool,
            stableford::SaveInput {
                request_id: request,
                round_id: r,
                hole_id: h,
                owner: ScoreOwner::Player { id: p },
                input: Input::NoScore {},
                expected_score: ExpectedScore::Absent {},
                session_id: f.base.session,
            },
        )
        .await
        .unwrap();
    }
    assert_ne!(before, source(&pool, &f, r, OwnerKind::Player, p).await);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT generation FROM fantasy_owner_generations WHERE round_id=$1 AND owner_id=$2"
        )
        .bind(r)
        .bind(p)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn both_fourball_partners_invalidate_same_side_token(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let team = Uuid::new_v4();
    sqlx::query("UPDATE rounds SET scoring_format='four_ball_stroke_play' WHERE id=$1")
        .bind(r)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams(id,round_id,tournament_id,name) VALUES($1,$2,$3,'Partners')")
        .bind(team)
        .bind(r)
        .bind(f.base.tournament)
        .execute(&pool)
        .await
        .unwrap();
    for p in &f.players[..2] {
        sqlx::query("INSERT INTO team_memberships(team_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(team).bind(r).bind(f.base.tournament).bind(p).execute(&pool).await.unwrap();
    }
    source_fixture(&pool, &f, r).await;
    let mut before = source(&pool, &f, r, OwnerKind::Team, team).await;
    let h = hole(&pool, r).await;
    for p in &f.players[..2] {
        four_ball::save_conditional(
            &pool,
            four_ball::SaveInput {
                request_id: Uuid::new_v4(),
                round_id: r,
                hole_id: h,
                owner: ScoreOwner::Player { id: *p },
                input: Input::Numeric { gross_strokes: 4 },
                expected_score: ExpectedScore::Absent {},
                session_id: f.base.session,
            },
        )
        .await
        .unwrap();
        let after = source(&pool, &f, r, OwnerKind::Team, team).await;
        assert_ne!(before, after);
        before = after;
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT generation FROM fantasy_owner_generations WHERE round_id=$1 AND owner_kind='team' AND owner_id=$2").bind(r).bind(team).fetch_one(&pool).await.unwrap(),2);
    assert!(
        fantasy::source(
            &pool,
            f.base.session,
            f.base.tournament,
            r,
            OwnerKind::Player,
            f.players[0]
        )
        .await
        .is_err()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn accepted_match_result_confirmation_and_correction_invalidate_both_players(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let a = f.players[0];
    let b = f.players[1];
    let id = Uuid::new_v4();
    sqlx::query("UPDATE rounds SET scoring_format='singles_match_play' WHERE id=$1")
        .bind(r)
        .execute(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('app.match_actor',$1,true),set_config('app.match_session',$2,true)",
    )
    .bind(f.base.admin.to_string())
    .bind(f.base.session.to_string())
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("INSERT INTO singles_matches(id,round_id,tournament_id,first_player_id,second_player_id) VALUES($1,$2,$3,$4,$5)").bind(id).bind(r).bind(f.base.tournament).bind(a).bind(b).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    source_fixture(&pool, &f, r).await;
    let before_a = source(&pool, &f, r, OwnerKind::Player, a).await;
    let before_b = source(&pool, &f, r, OwnerKind::Player, b).await;
    fantasy::dispose(
        &pool,
        f.base.session,
        f.base.tournament,
        r,
        OwnerKind::Player,
        a,
        &Dispose {
            expected_source_token: before_a.clone(),
            disposed: true,
            correction: false,
            reason: "No result yet".into(),
        },
    )
    .await
    .unwrap();
    match_play::execute(
        &pool,
        f.base.session,
        r,
        id,
        MatchRequest {
            request_id: Uuid::new_v4(),
            expected_revision: ScoreRevision::from_database(1).unwrap(),
            command: Command::Report {
                event: Event::Concession {
                    conceding_player_id: b,
                    communicated: true,
                    after_hole: 0,
                },
            },
        },
    )
    .await
    .unwrap();
    let report_a = source(&pool, &f, r, OwnerKind::Player, a).await;
    assert_ne!(before_a, report_a);
    assert_ne!(before_b, source(&pool, &f, r, OwnerKind::Player, b).await);
    let stale = fantasy::source(
        &pool,
        f.base.session,
        f.base.tournament,
        r,
        OwnerKind::Player,
        a,
    )
    .await
    .unwrap();
    assert!(!stale.disposition_current);
    match_play::execute(
        &pool,
        f.base.session,
        r,
        id,
        MatchRequest {
            request_id: Uuid::new_v4(),
            expected_revision: ScoreRevision::from_database(2).unwrap(),
            command: Command::Confirm {
                result_agreed_or_awarded: true,
            },
        },
    )
    .await
    .unwrap();
    let confirmed = source(&pool, &f, r, OwnerKind::Player, a).await;
    assert_ne!(report_a, confirmed);
    let event: Uuid =
        sqlx::query_scalar("SELECT (ledger->0->>'id')::uuid FROM singles_matches WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    match_play::execute(
        &pool,
        f.base.session,
        r,
        id,
        MatchRequest {
            request_id: Uuid::new_v4(),
            expected_revision: ScoreRevision::from_database(3).unwrap(),
            command: Command::Correct {
                kind: CorrectionKind::RecordingError,
                reason: "Correct concession attribution".into(),
                superseded_event_ids: vec![event],
                replacement: vec![Event::Concession {
                    conceding_player_id: a,
                    communicated: true,
                    after_hole: 0,
                }],
            },
        },
    )
    .await
    .unwrap();
    assert_ne!(confirmed, source(&pool, &f, r, OwnerKind::Player, a).await);
    assert!(
        fantasy::dispose(
            &pool,
            f.base.session,
            f.base.tournament,
            r,
            OwnerKind::Player,
            a,
            &Dispose {
                expected_source_token: source(&pool, &f, r, OwnerKind::Player, a).await,
                disposed: true,
                correction: true,
                reason: "Cannot override accepted result".into()
            }
        )
        .await
        .is_err()
    );
}
