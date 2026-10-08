#![cfg(feature = "database-tests")]
mod fantasy_results_support;
use fantasy_results_support::*;
async fn side(pool: &PgPool, f: &Fantasy, r: Uuid, players: &[Uuid]) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO teams(id,round_id,tournament_id,name) VALUES($1,$2,$3,$4)")
        .bind(id)
        .bind(r)
        .bind(f.base.tournament)
        .bind(id.to_string())
        .execute(pool)
        .await
        .unwrap();
    for p in players {
        sqlx::query("INSERT INTO team_memberships(team_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(id).bind(r).bind(f.base.tournament).bind(p).execute(pool).await.unwrap();
    }
    id
}
async fn four_input(pool: &PgPool, f: &Fantasy, r: Uuid, p: Uuid, h: Uuid, input: Input) {
    four_ball::save_conditional(
        pool,
        four_ball::SaveInput {
            request_id: Uuid::new_v4(),
            round_id: r,
            hole_id: h,
            owner: ScoreOwner::Player { id: p },
            input,
            expected_score: ExpectedScore::Absent {},
            session_id: f.base.session,
        },
    )
    .await
    .unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn stableford_native_ranking_uncapped_penalties_and_pickup(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    sqlx::query("UPDATE rounds SET scoring_format='individual_stableford',handicap_enabled=false WHERE id=$1").bind(r).execute(&pool).await.unwrap();
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    let h = holes(&pool, r).await;
    for (index, p) in f.players.iter().enumerate() {
        if index > 2 {
            disposition(&pool, &f, r, OwnerKind::Player, *p, false).await;
            continue;
        }
        for (number, hole) in h.iter().enumerate() {
            let input = if index == 2 && number == 0 {
                Input::NoScore {}
            } else {
                Input::Numeric {
                    gross_strokes: if index == 1 && number == 0 {
                        20
                    } else if index == 1 && number < 3 {
                        3
                    } else {
                        4
                    },
                }
            };
            stableford::save_conditional(
                &pool,
                stableford::SaveInput {
                    request_id: Uuid::new_v4(),
                    round_id: r,
                    hole_id: *hole,
                    owner: ScoreOwner::Player { id: *p },
                    input,
                    expected_score: ExpectedScore::Absent {},
                    session_id: f.base.session,
                },
            )
            .await
            .unwrap();
        }
        stableford::confirm(&pool, f.base.session, r, *p)
            .await
            .unwrap();
    }
    let value = scores(&pool, &f, r, f.base.session).await;
    // First two both have native36 despite very different raw stroke sums.
    assert_eq!(golfer(&value, f.players[0])["placement_points"], 10);
    assert_eq!(golfer(&value, f.players[1])["placement_points"], 10);
    assert_eq!(golfer(&value, f.players[1])["holes"][0]["net_strokes"], 20);
    assert_eq!(
        golfer(&value, f.players[1])["holes"][0]["points"]["total"],
        -5
    );
    assert_eq!(golfer(&value, f.players[1])["points"]["total"], 7);
    assert_eq!(
        golfer(&value, f.players[2])["holes"][0]["category"],
        "pickup"
    );
    assert_eq!(golfer(&value, f.players[2])["placement_points"], 6);
    assert_eq!(golfer(&value, f.players[2])["points"]["total"], 1);
}
#[sqlx::test(migrations = "../migrations")]
async fn fourball_pickups_pending_partner_and_noncounting_ace(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    sqlx::query("UPDATE rounds SET scoring_format='four_ball_stroke_play' WHERE id=$1")
        .bind(r)
        .execute(&pool)
        .await
        .unwrap();
    let team = side(&pool, &f, r, &f.players[..2]).await;
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    snapshot(&pool, r, f.players[0], -36, -36).await;
    snapshot(&pool, r, f.players[1], 18, 18).await;
    let h = holes(&pool, r).await;
    four_input(
        &pool,
        &f,
        r,
        f.players[0],
        h[0],
        Input::Numeric { gross_strokes: 1 },
    )
    .await;
    let pending = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&pending, f.players[0])["holes"][0]["points"]["state"],
        "pending"
    );
    four_input(
        &pool,
        &f,
        r,
        f.players[1],
        h[0],
        Input::Numeric { gross_strokes: 2 },
    )
    .await;
    for hole in &h[1..] {
        for p in &f.players[..2] {
            four_input(&pool, &f, r, *p, *hole, Input::NoScore {}).await;
        }
    }
    let value = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(golfer(&value, f.players[0])["holes"][0]["category"], "ace");
    assert_eq!(golfer(&value, f.players[0])["holes"][0]["net_strokes"], 1);
    assert_eq!(
        golfer(&value, f.players[0])["holes"][1]["category"],
        "pickup"
    );
    assert_eq!(
        golfer(&value, f.players[0])["points"],
        json!({"state":"pending","recorded":-75})
    );
    assert!(golfer(&value, f.players[0])["placement_points"].is_null());
    disposition(&pool, &f, r, OwnerKind::Team, team, false).await;
    let value = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&value, f.players[0])["points"],
        json!({"state":"settled","total":-75})
    );
    assert_eq!(
        golfer(&value, f.players[0])["points"],
        golfer(&value, f.players[1])["points"]
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn fourball_sporting_confirmation_does_not_resolve_missing_partner(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    sqlx::query("UPDATE rounds SET scoring_format='four_ball_stroke_play',handicap_enabled=false WHERE id=$1").bind(r).execute(&pool).await.unwrap();
    let team = side(&pool, &f, r, &f.players[..2]).await;
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    for h in holes(&pool, r).await {
        four_input(
            &pool,
            &f,
            r,
            f.players[0],
            h,
            Input::Numeric { gross_strokes: 4 },
        )
        .await;
    }
    four_ball::confirm(&pool, f.base.session, r, team)
        .await
        .unwrap();
    let value = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(golfer(&value, f.players[0])["points"]["state"], "pending");
    assert!(golfer(&value, f.players[0])["placement_points"].is_null());
}
#[sqlx::test(migrations = "../migrations")]
async fn scramble_and_foursomes_use_distinct_frozen_team_handicaps(pool: PgPool) {
    let f = fixture(&pool).await;
    for (index, format) in ["team_scramble", "two_player_foursomes"].iter().enumerate() {
        let r = f.rounds[index];
        sqlx::query("UPDATE rounds SET scoring_format=$2::scoring_format,handicap_allowance_percent=$3 WHERE id=$1").bind(r).bind(format).bind(if index==0{100i16}else{50i16}).execute(&pool).await.unwrap();
        let team = side(&pool, &f, r, &f.players[..2]).await;
        close(&pool, &f, r).await;
        source_fixture(&pool, &f, r).await;
        snapshot(&pool, r, f.players[0], 10, 40).await;
        snapshot(&pool, r, f.players[1], 30, 40).await;
        if index == 1 {
            let mut tx = pool.begin().await.unwrap();
            sqlx::query("ALTER TABLE round_team_handicap_snapshots DISABLE TRIGGER USER")
                .execute(&mut *tx)
                .await
                .unwrap();
            sqlx::query("INSERT INTO round_team_handicap_snapshots(round_id,tournament_id,team_id,playing_handicap) VALUES($1,$2,$3,11)").bind(r).bind(f.base.tournament).bind(team).execute(&mut *tx).await.unwrap();
            sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
                .execute(&mut *tx)
                .await
                .unwrap();
            sqlx::query("ALTER TABLE round_team_handicap_snapshots ENABLE TRIGGER USER")
                .execute(&mut *tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
        fill(&pool, &f, r, ScoreOwner::Team { id: team }, 4).await;
        let value = scores(&pool, &f, r, f.base.session).await;
        assert_eq!(
            golfer(&value, f.players[0])["recorded_hole_points"],
            if index == 0 { 8 } else { 11 }
        );
        assert_eq!(
            golfer(&value, f.players[0])["points"]["total"],
            if index == 0 { 18 } else { 21 }
        );
        assert_eq!(
            golfer(&value, f.players[0])["points"],
            golfer(&value, f.players[1])["points"]
        );
    }
}
