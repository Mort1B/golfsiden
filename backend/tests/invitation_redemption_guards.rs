#![cfg(feature = "database-tests")]

use chrono::{Duration, Utc};
use golf_api::auth::hash_invitation_token;
use sqlx::PgPool;
use uuid::Uuid;

struct GuardSeed {
    tournament_id: Uuid,
    invitation_id: Uuid,
    admin_id: Uuid,
    users: Vec<(Uuid, Uuid)>,
}

async fn seed(pool: &PgPool, user_count: usize, maximum: Option<i32>) -> GuardSeed {
    let tournament_id = Uuid::new_v4();
    let invitation_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, display_name, role)
         VALUES ($1, $2, 'Guard admin', 'viewer')",
    )
    .bind(admin_id)
    .bind(format!("{admin_id}@guard.test"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds, status)
         VALUES ($1, 'Guard trip', '2026-09-01', '2026-09-02', 1, 'draft')",
    )
    .bind(tournament_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin')",
    )
    .bind(tournament_id)
    .bind(admin_id)
    .execute(pool)
    .await
    .unwrap();
    let mut users = Vec::new();
    for index in 0..user_count {
        let user_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO players (id, display_name, current_handicap_index)
             VALUES ($1, $2, 10.0)",
        )
        .bind(player_id)
        .bind(format!("Guard {index}"))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO users (id, email, display_name, role, player_id)
             VALUES ($1, $2, $3, 'player', $4)",
        )
        .bind(user_id)
        .bind(format!("{user_id}@guard.test"))
        .bind(format!("Guard {index}"))
        .bind(player_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tournament_memberships (tournament_id, user_id, role)
             VALUES ($1, $2, 'player')",
        )
        .bind(tournament_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tournament_players
               (tournament_id, player_id, tournament_handicap)
             VALUES ($1, $2, 10.0)",
        )
        .bind(tournament_id)
        .bind(player_id)
        .execute(pool)
        .await
        .unwrap();
        users.push((user_id, player_id));
    }
    sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at,
            max_uses, series_id)
         VALUES ($1, $2, $3, $4, $5, $6, $1)",
    )
    .bind(invitation_id)
    .bind(tournament_id)
    .bind(hash_invitation_token("guard-token").as_slice())
    .bind(admin_id)
    .bind(Utc::now() + Duration::days(1))
    .bind(maximum)
    .execute(pool)
    .await
    .unwrap();
    GuardSeed {
        tournament_id,
        invitation_id,
        admin_id,
        users,
    }
}

async fn direct_redeem(
    pool: &PgPool,
    invitation_id: Uuid,
    series_id: Uuid,
    tournament_id: Uuid,
    user_id: Uuid,
    player_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO invitation_redemptions
           (id, invitation_id, series_id, tournament_id, user_id, player_id, mode)
         VALUES ($1, $2, $3, $4, $5, $6, 'acceptance')",
    )
    .bind(Uuid::new_v4())
    .bind(invitation_id)
    .bind(series_id)
    .bind(tournament_id)
    .bind(user_id)
    .bind(player_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn constraint(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .and_then(|error| error.constraint())
}

#[sqlx::test(migrations = "../migrations")]
async fn direct_sql_rejects_invalid_lifecycle_linkage_and_capacity(pool: PgPool) {
    let seed = seed(&pool, 2, Some(1)).await;
    let first = seed.users[0];
    let second = seed.users[1];
    direct_redeem(
        &pool,
        seed.invitation_id,
        seed.invitation_id,
        seed.tournament_id,
        first.0,
        first.1,
    )
    .await
    .unwrap();
    let exhausted = direct_redeem(
        &pool,
        seed.invitation_id,
        seed.invitation_id,
        seed.tournament_id,
        second.0,
        second.1,
    )
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&exhausted),
        Some("invitation_redemption_capacity_exhausted")
    );

    for (state, expected) in [
        ("revoked", "invitation_redemption_revoked"),
        ("expired", "invitation_redemption_expired"),
        ("closed", "invitation_redemption_tournament_closed"),
    ] {
        let invitation_id = Uuid::new_v4();
        let (created_at, expires_at) = if state == "expired" {
            (
                Utc::now() - Duration::days(2),
                Utc::now() - Duration::days(1),
            )
        } else {
            (Utc::now(), Utc::now() + Duration::days(1))
        };
        sqlx::query(
            "INSERT INTO tournament_invitations
               (id, tournament_id, token_hash, created_by_user_id, created_at,
                expires_at, series_id, revoked_at, revoked_by_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $1,
                     CASE WHEN $7 THEN clock_timestamp() ELSE NULL END,
                     CASE WHEN $7 THEN $4 ELSE NULL END)",
        )
        .bind(invitation_id)
        .bind(seed.tournament_id)
        .bind(hash_invitation_token(state).as_slice())
        .bind(seed.admin_id)
        .bind(created_at)
        .bind(expires_at)
        .bind(state == "revoked")
        .execute(&pool)
        .await
        .unwrap();
        if state == "closed" {
            sqlx::query("UPDATE tournaments SET status = 'completed' WHERE id = $1")
                .bind(seed.tournament_id)
                .execute(&pool)
                .await
                .unwrap();
        }
        let error = direct_redeem(
            &pool,
            invitation_id,
            invitation_id,
            seed.tournament_id,
            second.0,
            second.1,
        )
        .await
        .unwrap_err();
        assert_eq!(constraint(&error), Some(expected));
        if state == "closed" {
            sqlx::query("UPDATE tournaments SET status = 'draft' WHERE id = $1")
                .bind(seed.tournament_id)
                .execute(&pool)
                .await
                .unwrap();
        }
    }

    let bad_link = direct_redeem(
        &pool,
        seed.invitation_id,
        Uuid::new_v4(),
        seed.tournament_id,
        second.0,
        second.1,
    )
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&bad_link),
        Some("invitation_redemption_target_invalid")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn direct_sql_requires_exact_identity_membership_and_entrant_links(pool: PgPool) {
    let seed = seed(&pool, 2, None).await;
    let first = seed.users[0];
    let second = seed.users[1];

    let mismatched_identity = direct_redeem(
        &pool,
        seed.invitation_id,
        seed.invitation_id,
        seed.tournament_id,
        first.0,
        second.1,
    )
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&mismatched_identity),
        Some("invitation_redemption_user_player_invalid")
    );

    sqlx::query(
        "DELETE FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2",
    )
    .bind(seed.tournament_id)
    .bind(second.0)
    .execute(&pool)
    .await
    .unwrap();
    let missing_membership = direct_redeem(
        &pool,
        seed.invitation_id,
        seed.invitation_id,
        seed.tournament_id,
        second.0,
        second.1,
    )
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&missing_membership),
        Some("invitation_redemption_membership_missing")
    );

    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'player')",
    )
    .bind(seed.tournament_id)
    .bind(second.0)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "DELETE FROM tournament_players
         WHERE tournament_id = $1 AND player_id = $2",
    )
    .bind(seed.tournament_id)
    .bind(second.1)
    .execute(&pool)
    .await
    .unwrap();
    let missing_entrant = direct_redeem(
        &pool,
        seed.invitation_id,
        seed.invitation_id,
        seed.tournament_id,
        second.0,
        second.1,
    )
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&missing_entrant),
        Some("invitation_redemption_entrant_missing")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_direct_sql_cannot_over_redeem_last_slot(pool: PgPool) {
    let seed = seed(&pool, 2, Some(1)).await;
    let first_pool = pool.clone();
    let second_pool = pool.clone();
    let first = seed.users[0];
    let second = seed.users[1];
    let (first_result, second_result) = tokio::join!(
        direct_redeem(
            &first_pool,
            seed.invitation_id,
            seed.invitation_id,
            seed.tournament_id,
            first.0,
            first.1,
        ),
        direct_redeem(
            &second_pool,
            seed.invitation_id,
            seed.invitation_id,
            seed.tournament_id,
            second.0,
            second.1,
        ),
    );
    assert_ne!(first_result.is_ok(), second_result.is_ok());
    let error = first_result.err().or_else(|| second_result.err()).unwrap();
    assert_eq!(
        constraint(&error),
        Some("invitation_redemption_capacity_exhausted")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM invitation_redemptions
             WHERE tournament_id = $1 AND series_id = $2",
        )
        .bind(seed.tournament_id)
        .bind(seed.invitation_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}
