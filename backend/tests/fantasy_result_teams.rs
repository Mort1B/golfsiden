#![cfg(feature = "database-tests")]
mod fantasy_results_support;
use fantasy_results_support::*;
async fn teams(pool: &PgPool, f: &Fantasy, r: Uuid, pairs: &[[Uuid; 2]]) -> Vec<Uuid> {
    sqlx::query(
        "UPDATE rounds SET scoring_format='team_scramble',handicap_enabled=false WHERE id=$1",
    )
    .bind(r)
    .execute(pool)
    .await
    .unwrap();
    let mut teams = vec![];
    for pair in pairs {
        let team = Uuid::new_v4();
        sqlx::query("INSERT INTO teams(id,round_id,tournament_id,name) VALUES($1,$2,$3,$4)")
            .bind(team)
            .bind(r)
            .bind(f.base.tournament)
            .bind(team.to_string())
            .execute(pool)
            .await
            .unwrap();
        for p in pair {
            sqlx::query("INSERT INTO team_memberships(team_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(team).bind(r).bind(f.base.tournament).bind(p).execute(pool).await.unwrap();
        }
        teams.push(team);
    }
    teams
}
async fn ranked_scores(pool: &PgPool, f: &Fantasy, r: Uuid, teams: &[Uuid]) {
    let h = holes(pool, r).await;
    for (index, team) in teams.iter().enumerate() {
        for (number, hole) in h.iter().enumerate() {
            let extra = if number == 0 {
                index.min(16)
            } else if number == 1 {
                index.saturating_sub(16)
            } else {
                0
            };
            numeric(
                pool,
                f,
                r,
                ScoreOwner::Team { id: *team },
                *hole,
                4 + extra as i16,
            )
            .await;
        }
        scorecards::confirm_authenticated(pool, r, ScoreOwner::Team { id: *team }, f.base.session)
            .await
            .unwrap();
    }
}
async fn field(pool: PgPool, count: usize) {
    let mut f = fixture(&pool).await;
    while f.players.len() < count * 2 {
        f.players.push(prepared(&pool, &f.base).await.0.player_id);
    }
    // Additional existing entries exercise one-time closure cost without generating picks.
    for _ in 2..count * 2 {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES($1,$2,'Fantasy manager','viewer')").bind(id).bind(id.simple().to_string()).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'viewer')",
        )
        .bind(f.base.tournament)
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO fantasy_entries(tournament_id,user_id) VALUES($1,$2)")
            .bind(f.base.tournament)
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    let r = f.rounds[0];
    let pairs = f.players[..count * 2].as_chunks::<2>().0.to_vec();
    let teams = teams(&pool, &f, r, &pairs).await;
    fantasy::save(&pool, f.base.session, f.base.tournament, r, &lineup(&f))
        .await
        .unwrap();
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    ranked_scores(&pool, &f, r, &teams).await;
    let started = std::time::Instant::now();
    let first = scores(&pool, &f, r, f.base.session).await;
    let first_elapsed = started.elapsed();
    let started = std::time::Instant::now();
    let again = scores(&pool, &f, r, f.base.session).await;
    let repeat_elapsed = started.elapsed();
    assert_eq!(first, again);
    eprintln!(
        "Fantasy projection field: teams={count}, golfers={}, manager_entries={}, first_read_ms={}, repeated_read_ms={}",
        f.players.len(),
        count * 2,
        first_elapsed.as_millis(),
        repeat_elapsed.as_millis()
    );
    for (index, pair) in pairs.iter().enumerate() {
        let award = [10, 8, 6, 5, 4, 3, 2, 1].get(index).copied().unwrap_or(0);
        assert_eq!(golfer(&first, pair[0])["placement_points"], award);
        assert_eq!(
            golfer(&first, pair[0])["points"],
            golfer(&first, pair[1])["points"]
        );
    }
    assert_eq!(manager(&first, f.base.admin)["points"]["total"], 44);
    assert_eq!(first["golfers"].as_array().unwrap().len(), f.players.len());
}
#[sqlx::test(migrations = "../migrations")]
async fn nine_teams_are_nine_placing_units_and_eighteen_golfers(pool: PgPool) {
    field(pool, 9).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn larger_field_and_closure_read_cost(pool: PgPool) {
    field(pool, 20).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn changing_partners_carry_forward_all_rounds_and_historical_withdrawal(pool: PgPool) {
    let f = fixture(&pool).await;
    let p = &f.players;
    let pairs = [[p[0], p[1]], [p[2], p[3]]];
    let r = f.rounds[0];
    let first_teams = teams(&pool, &f, r, &pairs).await;
    fantasy::save(&pool, f.base.session, f.base.tournament, r, &lineup(&f))
        .await
        .unwrap();
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    ranked_scores(&pool, &f, r, &first_teams).await;
    let second = f.rounds[1];
    let changed = [[p[0], p[2]], [p[1], p[3]]];
    let second_teams = teams(&pool, &f, second, &changed).await;
    close(&pool, &f, second).await;
    source_fixture(&pool, &f, second).await;
    ranked_scores(&pool, &f, second, &second_teams).await;
    let view = scores(&pool, &f, second, f.base.session).await;
    assert_eq!(
        manager(&view, f.base.admin)["lineup"]["origin"],
        "carried_forward"
    );
    assert_eq!(
        manager(&view, f.base.admin)["lineup"]["captain"],
        p[1].to_string()
    );
    assert_eq!(manager(&view, f.base.admin)["points"]["total"], 41);
    let overall = serde_json::to_value(
        results::overall(&pool, f.base.session, f.base.tournament)
            .await
            .unwrap(),
    )
    .unwrap();
    let manager = overall["managers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == f.base.admin.to_string())
        .unwrap();
    assert_eq!(manager["points"], json!({"state":"provisional","total":85}));
    let player = overall["golfers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == p[1].to_string())
        .unwrap();
    assert_eq!(player["points"]["total"], 17);
    // Existing sporting withdrawal requires historical participating rounds locked.
    for round in [r, second] {
        historical_status(&pool, round, p[1], "locked").await;
    }
    player_claims::withdraw(&pool, f.base.session, f.base.tournament, p[1])
        .await
        .unwrap();
    let after = serde_json::to_value(
        results::overall(&pool, f.base.session, f.base.tournament)
            .await
            .unwrap(),
    )
    .unwrap();
    let historical = after["golfers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == p[1].to_string())
        .unwrap();
    assert_eq!(historical["points"]["total"], 17);
}
