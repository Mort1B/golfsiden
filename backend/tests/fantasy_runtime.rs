#![cfg(feature = "database-tests")]
mod fantasy_support;
use fantasy_support::*;

#[sqlx::test(migrations = "../migrations")]
async fn restricted_runtime_role_saves_carries_and_records_dispositions(pool: PgPool) {
    let f = fixture(&pool).await;
    // Generated identifier contains only a constant prefix and hexadecimal UUID.
    let role = format!("fantasy_runtime_{}", Uuid::new_v4().simple());
    sqlx::query(&format!(
        "CREATE ROLE {role} NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT"
    ))
    .execute(&pool)
    .await
    .unwrap();
    for grant in [
        "GRANT USAGE ON SCHEMA public",
        "GRANT SELECT,INSERT,UPDATE,DELETE ON ALL TABLES IN SCHEMA public",
        "GRANT USAGE,SELECT ON ALL SEQUENCES IN SCHEMA public",
        "GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public",
    ] {
        sqlx::query(&format!("{grant} TO {role}"))
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query(&format!(
        "REVOKE INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER ON _sqlx_migrations FROM {role}"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let set_role = format!("SET ROLE {role}");
    let runtime = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .after_connect(move |connection, _| {
            let sql = set_role.clone();
            Box::pin(async move {
                sqlx::query(&sql).execute(connection).await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let elevated: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolcreatedb OR rolcreaterole FROM pg_roles WHERE rolname=current_user",
    )
    .fetch_one(&runtime)
    .await
    .unwrap();
    assert!(!elevated);
    fantasy::configure(&runtime, f.base.session, f.base.tournament, true)
        .await
        .unwrap();
    fantasy::enter(&runtime, f.base.session, f.base.tournament)
        .await
        .unwrap();
    let input = lineup(&f);
    let first = fantasy::save(
        &runtime,
        f.base.session,
        f.base.tournament,
        f.rounds[0],
        &input,
    )
    .await
    .unwrap();
    close(&runtime, &f, f.rounds[0]).await;
    read(&runtime, &f, f.rounds[0]).await;
    close(&runtime, &f, f.rounds[1]).await;
    let carried = read(&runtime, &f, f.rounds[1]).await;
    let receipt = carried
        .selections
        .iter()
        .find(|s| s.user_id == f.base.admin)
        .unwrap()
        .receipt
        .as_ref()
        .unwrap();
    assert_eq!(receipt.origin, "carried_forward");
    assert_eq!(receipt.captain, first.captain);
    ready_round(&pool, &f, f.rounds[1]).await;
    golf_api::repositories::round_lifecycle::open_authorized(&runtime, f.base.session, f.rounds[1])
        .await
        .unwrap();
    let source = fantasy::source(
        &runtime,
        f.base.session,
        f.base.tournament,
        f.rounds[1],
        OwnerKind::Player,
        f.players[0],
    )
    .await
    .unwrap();
    let disposition = fantasy::dispose(
        &runtime,
        f.base.session,
        f.base.tournament,
        f.rounds[1],
        OwnerKind::Player,
        f.players[0],
        &Dispose {
            expected_source_token: source.source_token,
            disposed: true,
            correction: false,
            reason: "Recorded non-finish".into(),
        },
    )
    .await
    .unwrap();
    assert!(disposition.disposition_current);
    let hole: Uuid = sqlx::query_scalar("SELECT h.id FROM holes h JOIN rounds r ON r.tee_id=h.tee_id WHERE r.id=$1 ORDER BY h.hole_number LIMIT 1")
        .bind(f.rounds[1]).fetch_one(&runtime).await.unwrap();
    golf_api::repositories::scorecards::save_authenticated(
        &runtime,
        golf_api::repositories::scorecards::AuthenticatedSaveScore {
            round_id: f.rounds[1],
            hole_id: hole,
            owner: golf_api::domain::scorecards::ScoreOwner::Player { id: f.players[0] },
            gross_strokes: 4,
            session_id: f.base.session,
        },
    )
    .await
    .unwrap();
    let changed = fantasy::source(
        &runtime,
        f.base.session,
        f.base.tournament,
        f.rounds[1],
        OwnerKind::Player,
        f.players[0],
    )
    .await
    .unwrap();
    assert_ne!(changed.source_token, disposition.source_token);
    assert!(!changed.disposition_current);
    assert!(
        sqlx::query("UPDATE fantasy_rounds SET opened_at=NULL WHERE round_id=$1")
            .bind(f.rounds[1])
            .execute(&runtime)
            .await
            .is_err()
    );
    assert!(sqlx::query("INSERT INTO fantasy_membership_history(tournament_id,user_id,present) VALUES($1,$2,false)")
        .bind(f.base.tournament).bind(f.base.admin).execute(&runtime).await.is_err());
    assert!(
        sqlx::query("DELETE FROM _sqlx_migrations")
            .execute(&runtime)
            .await
            .is_err()
    );
    runtime.close().await;
    sqlx::query(&format!("DROP OWNED BY {role}"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(&format!("DROP ROLE {role}"))
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = false)]
async fn schema34_upgrade_preserves_seeded_tournament_and_repeats(pool: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            golf_api::schema::MIGRATOR
                .iter()
                .filter(|m| m.version <= 34)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&pool).await.unwrap();
    sqlx::raw_sql(include_str!("../seed.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let fingerprint = "SELECT jsonb_build_object('players',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM players p),'rounds',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r),'tournaments',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tournaments t))";
    let before: serde_json::Value = sqlx::query_scalar(fingerprint)
        .fetch_one(&pool)
        .await
        .unwrap();
    golf_api::schema::MIGRATOR.run(&pool).await.unwrap();
    let after: serde_json::Value = sqlx::query_scalar(fingerprint)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    golf_api::schema::MIGRATOR.run(&pool).await.unwrap();
    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../seed.sql"))
            .execute(&pool)
            .await
            .unwrap();
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        37
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_games")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
