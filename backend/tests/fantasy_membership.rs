#![cfg(feature = "database-tests")]
mod fantasy_support;
use fantasy_support::*;

#[sqlx::test(migrations = "../migrations")]
async fn membership_identity_move_records_both_sides_at_close(pool: PgPool) {
    let f = fixture(&pool).await;
    let recipient = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,display_name,password_hash) VALUES($1,'recipient','Recipient','unused')")
        .bind(recipient)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE tournament_memberships SET user_id=$1 WHERE tournament_id=$2 AND user_id=$3",
    )
    .bind(recipient)
    .bind(f.base.tournament)
    .bind(f.base.other)
    .execute(&pool)
    .await
    .unwrap();
    for (user, expected) in [(f.base.other, false), (recipient, true)] {
        let present: bool = sqlx::query_scalar("SELECT present FROM fantasy_membership_history WHERE tournament_id=$1 AND user_id=$2 ORDER BY effective_at DESC,id DESC LIMIT 1")
            .bind(f.base.tournament).bind(user).fetch_one(&pool).await.unwrap();
        assert_eq!(present, expected);
    }
    close(&pool, &f, f.rounds[0]).await;
    read(&pool, &f, f.rounds[0]).await;
    let state: String =
        sqlx::query_scalar("SELECT state FROM fantasy_selections WHERE round_id=$1 AND user_id=$2")
            .bind(f.rounds[0])
            .bind(f.base.other)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state, "not_participating");
}
