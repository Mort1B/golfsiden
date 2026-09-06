use golf_api::auth::hash_session_token;
use sqlx::PgPool;
use uuid::Uuid;

// Historical migrations are tested with their original session columns and
// trusted start context, never with current-schema runtime authorization SQL.
pub async fn create_and_start(pool: &PgPool, user: Uuid, tournament: Uuid, token: &str) -> Uuid {
    let id = Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO user_sessions(id,user_id,token_hash,expires_at) VALUES($1,$2,$3,now()+interval '1 hour')")
        .bind(id).bind(user).bind(hash_session_token(token)).execute(&mut *tx).await.unwrap();
    sqlx::query("SELECT set_config('app.tournament_start_tournament_id',$1::text,true),set_config('app.tournament_start_user_id',$2::text,true)")
        .bind(tournament).bind(user).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE tournaments SET status='active' WHERE id=$1")
        .bind(tournament)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    id
}
