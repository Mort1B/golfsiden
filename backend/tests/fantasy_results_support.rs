#![allow(dead_code, unused_imports)]
#[path = "fantasy_support/mod.rs"]
mod foundation;
pub use foundation::*;
pub use golf_api::{
    domain::{
        four_ball_card::Input,
        scorecards::{ExpectedScore, ScoreOwner, ScoreRevision},
    },
    repositories::{
        fantasy::results, four_ball, match_play, scorecards, stableford, tournament_visibility,
    },
};
pub async fn holes(pool: &PgPool, r: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar("SELECT h.id FROM holes h JOIN rounds r ON r.tee_id=h.tee_id WHERE r.id=$1 ORDER BY hole_number").bind(r).fetch_all(pool).await.unwrap()
}
pub async fn numeric(pool: &PgPool, f: &Fantasy, r: Uuid, owner: ScoreOwner, h: Uuid, gross: i16) {
    scorecards::save_authenticated(
        pool,
        scorecards::AuthenticatedSaveScore {
            round_id: r,
            hole_id: h,
            owner,
            gross_strokes: gross,
            session_id: f.base.session,
        },
    )
    .await
    .unwrap();
}
pub async fn fill(pool: &PgPool, f: &Fantasy, r: Uuid, owner: ScoreOwner, gross: i16) {
    for h in holes(pool, r).await {
        numeric(pool, f, r, owner, h, gross).await;
    }
    scorecards::confirm_authenticated(pool, r, owner, f.base.session)
        .await
        .unwrap();
}
pub async fn scores(pool: &PgPool, f: &Fantasy, r: Uuid, session: Uuid) -> Value {
    serde_json::to_value(
        results::round_results(pool, session, f.base.tournament, r)
            .await
            .unwrap(),
    )
    .unwrap()
}
// Full result/breakdown paths retain tournament-wide fact loading. Their
// selected-round values must match the scoped loader exactly, including hidden
// fields, source provenance, contributions and revision-bearing summaries.
pub async fn assert_scoped_parity(pool: &PgPool, f: &Fantasy, r: Uuid, session: Uuid) {
    let scoped = results::round_results(pool, session, f.base.tournament, r)
        .await
        .unwrap();
    let overall = results::overall(pool, session, f.base.tournament)
        .await
        .unwrap();
    assert_eq!(scoped.rules_version, overall.rules_version);
    assert_eq!(
        serde_json::to_value(&scoped.round).unwrap(),
        serde_json::to_value(overall.rounds.iter().find(|s| s.round_id == r).unwrap()).unwrap()
    );
    for golfer in &scoped.golfers {
        let breakdown =
            results::golfer_breakdown(pool, session, f.base.tournament, golfer.player_id)
                .await
                .unwrap();
        assert_eq!(scoped.rules_version, breakdown.rules_version);
        let expected = breakdown
            .rounds
            .iter()
            .find(|s| s.round.round_id == r)
            .unwrap();
        assert_eq!(
            serde_json::to_value(golfer).unwrap(),
            serde_json::to_value(&expected.result).unwrap()
        );
    }
    for manager in &scoped.managers {
        let breakdown =
            results::manager_breakdown(pool, session, f.base.tournament, manager.user_id)
                .await
                .unwrap();
        assert_eq!(scoped.rules_version, breakdown.rules_version);
        let expected = breakdown
            .rounds
            .iter()
            .find(|s| s.round.round_id == r)
            .unwrap();
        assert_eq!(
            serde_json::to_value(manager).unwrap(),
            serde_json::to_value(&expected.result).unwrap()
        );
    }
}
pub fn golfer(value: &Value, id: Uuid) -> &Value {
    value["golfers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["player_id"] == id.to_string())
        .unwrap()
}
pub fn manager(value: &Value, id: Uuid) -> &Value {
    value["managers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["user_id"] == id.to_string())
        .unwrap()
}
pub async fn disposition(
    pool: &PgPool,
    f: &Fantasy,
    r: Uuid,
    kind: OwnerKind,
    owner: Uuid,
    correction: bool,
) {
    let source = fantasy::source(pool, f.base.session, f.base.tournament, r, kind, owner)
        .await
        .unwrap();
    fantasy::dispose(
        pool,
        f.base.session,
        f.base.tournament,
        r,
        kind,
        owner,
        &Dispose {
            expected_source_token: source.source_token,
            disposed: true,
            correction,
            reason: "Stopped playing".into(),
        },
    )
    .await
    .unwrap();
}
pub async fn hide(pool: &PgPool, f: &Fantasy, value: bool) {
    let current = tournament_visibility::get_for_admin(pool, f.base.admin, f.base.tournament)
        .await
        .unwrap();
    tournament_visibility::update_authorized(
        pool,
        f.base.session,
        f.base.tournament,
        value,
        current.visibility_updated_at,
    )
    .await
    .unwrap();
}
pub async fn draft_handicap_disabled(pool: &PgPool, r: Uuid) {
    sqlx::query("UPDATE rounds SET handicap_enabled=false WHERE id=$1")
        .bind(r)
        .execute(pool)
        .await
        .unwrap();
}
// Owner-only construction of explicit frozen fixture values, before scoring.
pub async fn snapshot(pool: &PgPool, r: Uuid, p: Uuid, course: i16, playing: i16) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("ALTER TABLE round_handicap_snapshots DISABLE TRIGGER USER")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE round_handicap_snapshots SET course_handicap=$3,playing_handicap=$4 WHERE round_id=$1 AND player_id=$2").bind(r).bind(p).bind(course).bind(playing).execute(&mut *tx).await.unwrap();
    sqlx::query("ALTER TABLE round_handicap_snapshots ENABLE TRIGGER USER")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}
