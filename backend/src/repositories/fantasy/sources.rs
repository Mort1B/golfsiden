use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;
use sha2::{Digest, Sha256};

// Canonical JSONB arrays are explicitly ordered. Every hole appears even when
// no input exists. This opaque token is returned only to exact administrators.
async fn fingerprint(
    tx: &mut Transaction<'_, Postgres>,
    r: Uuid,
    kind: OwnerKind,
    owner: Uuid,
) -> Result<[u8; 32], Error> {
    let valid:bool=sqlx::query_scalar("SELECT CASE WHEN $2='team' THEN EXISTS(SELECT 1 FROM teams t WHERE t.id=$3 AND t.round_id=r.id) AND r.scoring_format::text IN ('team_scramble','two_player_foursomes','four_ball_stroke_play') ELSE EXISTS(SELECT 1 FROM round_handicap_snapshots h WHERE h.round_id=r.id AND h.player_id=$3) AND r.scoring_format::text IN ('individual_stroke_play','individual_stableford','singles_match_play') END FROM rounds r WHERE r.id=$1").bind(r).bind(kind.as_str()).bind(owner).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(Error::Invalid);
    }
    let facts: Value = sqlx::query_scalar(include_str!("source_facts.sql"))
        .bind(r)
        .bind(kind.as_str())
        .bind(owner)
        .fetch_one(&mut **tx)
        .await?;
    let encoded = serde_json::to_vec(&facts).map_err(|_| Error::Invalid)?;
    Ok(Sha256::digest(encoded).into())
}
async fn latest(
    tx: &mut Transaction<'_, Postgres>,
    r: Uuid,
    kind: OwnerKind,
    owner: Uuid,
) -> Result<Option<Disposition>, Error> {
    Ok(sqlx::query_as("SELECT id,disposed,correction,reason,actor_id,created_at,source_token FROM fantasy_dispositions WHERE round_id=$1 AND (($2='player' AND player_id=$3) OR ($2='team' AND team_id=$3)) ORDER BY created_at DESC,id DESC LIMIT 1").bind(r).bind(kind.as_str()).bind(owner).fetch_optional(&mut **tx).await?)
}
pub async fn source(
    pool: &PgPool,
    session: Uuid,
    t: Uuid,
    r: Uuid,
    kind: OwnerKind,
    owner: Uuid,
) -> Result<Source, Error> {
    let (mut tx, _) = begin(pool, session, t, true).await?;
    enabled(&mut tx, t).await?;
    config::window(&mut tx, t, r).await?;
    let token = fingerprint(&mut tx, r, kind, owner).await?;
    let disposition = latest(&mut tx, r, kind, owner).await?;
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    Ok(Source {
        round_id: r,
        owner_kind: kind,
        owner_id: owner,
        source_token: URL_SAFE_NO_PAD.encode(token),
        disposition_current: disposition
            .as_ref()
            .is_some_and(|d| d.source_token.as_slice() == token),
        disposition,
    })
}
pub async fn dispose(
    pool: &PgPool,
    session: Uuid,
    t: Uuid,
    r: Uuid,
    kind: OwnerKind,
    owner: Uuid,
    input: &Dispose,
) -> Result<Source, Error> {
    if input.reason.trim().is_empty() || input.reason.len() > 500 || input.reason.contains('\0') {
        return Err(Error::Invalid);
    }
    let expected = URL_SAFE_NO_PAD
        .decode(&input.expected_source_token)
        .map_err(|_| Error::Invalid)?;
    let (mut tx, actor) = begin(pool, session, t, true).await?;
    enabled(&mut tx, t).await?;
    config::window(&mut tx, t, r).await?;
    let status: String = sqlx::query_scalar("SELECT status::text FROM rounds WHERE id=$1")
        .bind(r)
        .fetch_one(&mut *tx)
        .await?;
    let prior = latest(&mut tx, r, kind, owner).await?;
    if status == "draft" {
        return Err(Error::Unavailable);
    }
    if (status == "locked" || prior.is_some() || !input.disposed) && !input.correction {
        return Err(Error::Conflict);
    }
    let natural:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM scorecard_confirmations WHERE round_id=$1 AND (($2='player' AND player_id=$3) OR ($2='team' AND team_id=$3))) OR EXISTS(SELECT 1 FROM singles_matches WHERE round_id=$1 AND $2='player' AND (first_player_id=$3 OR second_player_id=$3) AND terminal)").bind(r).bind(kind.as_str()).bind(owner).fetch_one(&mut *tx).await?;
    if input.disposed && natural {
        return Err(Error::Conflict);
    }
    let token = fingerprint(&mut tx, r, kind, owner).await?;
    if expected.as_slice() != token {
        return Err(Error::Conflict);
    }
    sqlx::query("INSERT INTO fantasy_dispositions(id,tournament_id,round_id,player_id,team_id,source_token,disposed,correction,reason,actor_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)").bind(Uuid::new_v4()).bind(t).bind(r).bind(if kind==OwnerKind::Player{Some(owner)}else{None}).bind(if kind==OwnerKind::Team{Some(owner)}else{None}).bind(token.as_slice()).bind(input.disposed).bind(input.correction).bind(input.reason.trim()).bind(actor).execute(&mut *tx).await?;
    let disposition = latest(&mut tx, r, kind, owner).await?;
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    Ok(Source {
        round_id: r,
        owner_kind: kind,
        owner_id: owner,
        source_token: URL_SAFE_NO_PAD.encode(token),
        disposition_current: disposition
            .as_ref()
            .is_some_and(|d| d.source_token.as_slice() == token),
        disposition,
    })
}
