use golf_api::repositories::{round_completion, round_lifecycle};
use sqlx::PgPool;
use uuid::Uuid;

/// Historical upgrade fixtures must use their schema's persisted contract,
/// not current score DTOs that require columns introduced by later migrations.
pub async fn lock_player_round(
    pool: &PgPool,
    round: Uuid,
    tournament: Uuid,
    player: Uuid,
    actor: Uuid,
) {
    round_lifecycle::open(pool, round).await.unwrap();
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(round)
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true)")
        .bind(round)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO scores(id,round_id,tournament_id,hole_id,player_id,gross_strokes,submitted_by) SELECT gen_random_uuid(),r.id,$2,h.id,$3,4,$4 FROM rounds r JOIN holes h ON h.tee_id=r.tee_id WHERE r.id=$1 ORDER BY h.hole_number")
        .bind(round)
        .bind(tournament)
        .bind(player)
        .bind(actor)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO scorecard_confirmations(id,round_id,tournament_id,player_id,confirmed_by) VALUES(gen_random_uuid(),$1,$2,$3,$4)")
        .bind(round)
        .bind(tournament)
        .bind(player)
        .bind(actor)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    round_completion::complete(pool, round).await.unwrap();
    round_completion::lock(pool, round).await.unwrap();
}
