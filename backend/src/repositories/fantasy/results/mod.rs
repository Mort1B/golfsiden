mod load;
mod models;
mod round;
use super::*;
use crate::domain::fantasy::{
    RoundId,
    projection::{Contribution, rank_contributions},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
pub use models::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
fn revision<T: Serialize>(value: &T) -> Result<String, Error> {
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(
        serde_json::to_vec(value).map_err(|_| Error::Invalid)?,
    )))
}
enum Target {
    All,
    Round(Uuid),
    Golfer(Uuid),
    Manager(Uuid),
}
struct Bundle {
    overall: Results,
    rounds: Vec<RoundResult>,
}
async fn bundle(pool: &PgPool, session: Uuid, t: Uuid, target: Target) -> Result<Bundle, Error> {
    let mut pre = pool.begin().await?;
    authorize(&mut pre, session, t, false).await?;
    pre.rollback().await?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT id FROM rounds WHERE tournament_id=$1 ORDER BY id FOR UPDATE")
        .bind(t)
        .fetch_all(&mut *tx)
        .await?;
    let caller = authorize(&mut tx, session, t, false).await?;
    enabled(&mut tx, t).await?;
    let finalized = lifecycle::finalize(&mut tx, t).await?;
    let data = load::load(&mut tx, t, caller).await?;
    let exists = match target {
        Target::All => true,
        Target::Round(id) => data.rounds.iter().any(|r| r.id == id),
        Target::Golfer(id) => data.golfers.iter().any(|p| p.id == id),
        Target::Manager(id) => data.managers.iter().any(|m| m.id == id),
    };
    if !exists {
        return Err(AuthorizationError::NotFound.into());
    }
    let projected = data
        .rounds
        .iter()
        .map(|r| round::project(&data, r, t, caller))
        .collect::<Result<Vec<_>, _>>()?;
    let expected: Vec<_> = data.rounds.iter().map(|r| RoundId(r.id)).collect();
    let golfer_entries: Vec<_> = data
        .golfers
        .iter()
        .map(|g| {
            Ok((
                g.id,
                projected
                    .iter()
                    .map(|r| r.golfers.get(&g.id).copied().ok_or(Error::Invalid))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let manager_entries: Vec<_> = data
        .managers
        .iter()
        .map(|m| {
            Ok((
                m.id,
                projected
                    .iter()
                    .map(|r| r.managers.get(&m.id).copied().ok_or(Error::Invalid))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let golfers = overall_rows(
        &expected,
        &golfer_entries,
        &data
            .golfers
            .iter()
            .map(|g| (g.id, g.display_name.as_str()))
            .collect::<BTreeMap<_, _>>(),
    )?;
    let managers = overall_rows(
        &expected,
        &manager_entries,
        &data
            .managers
            .iter()
            .map(|g| (g.id, g.display_name.as_str()))
            .collect::<BTreeMap<_, _>>(),
    )?;
    let mut rounds: Vec<_> = projected.into_iter().map(|r| r.result).collect();
    for r in &mut rounds {
        r.revision = revision(r)?;
        r.finalized = finalized;
    }
    let mut overall = Results {
        tournament_id: t,
        rules_version: 1,
        revision: String::new(),
        rounds: rounds.iter().map(|r| r.round.clone()).collect(),
        golfers,
        managers,
        finalized,
    };
    overall.revision = revision(&overall)?;
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    Ok(Bundle { overall, rounds })
}
fn overall_rows(
    rounds: &[RoundId],
    entries: &[(Uuid, Vec<Contribution>)],
    names: &BTreeMap<Uuid, &str>,
) -> Result<Vec<Standing>, Error> {
    let ranks = rank_contributions(rounds, entries).map_err(|_| Error::Invalid)?;
    let mut rows = vec![];
    for (id, values) in entries {
        let (points, rank) = ranks.get(id).copied().ok_or(Error::Invalid)?;
        rows.push(Standing {
            id: *id,
            display_name: names.get(id).ok_or(Error::Invalid)?.to_string(),
            points: points.into(),
            rank,
            rounds: rounds
                .iter()
                .zip(values)
                .map(|(round, points)| RoundPoints {
                    round_id: round.0,
                    points: (*points).into(),
                })
                .collect(),
        });
    }
    rows.sort_by_key(|r| (r.rank.unwrap_or(usize::MAX), r.id));
    Ok(rows)
}
pub async fn overall(pool: &PgPool, session: Uuid, t: Uuid) -> Result<Results, Error> {
    Ok(bundle(pool, session, t, Target::All).await?.overall)
}
pub async fn round_results(
    pool: &PgPool,
    session: Uuid,
    t: Uuid,
    r: Uuid,
) -> Result<RoundResult, Error> {
    bundle(pool, session, t, Target::Round(r))
        .await?
        .rounds
        .into_iter()
        .find(|round| round.round.round_id == r)
        .ok_or(AuthorizationError::NotFound.into())
}
pub async fn golfer_breakdown(
    pool: &PgPool,
    session: Uuid,
    t: Uuid,
    p: Uuid,
) -> Result<GolferBreakdown, Error> {
    let b = bundle(pool, session, t, Target::Golfer(p)).await?;
    let standing = b
        .overall
        .golfers
        .into_iter()
        .find(|s| s.id == p)
        .ok_or(AuthorizationError::NotFound)?;
    let rounds = b
        .rounds
        .into_iter()
        .map(|r| {
            Ok(GolferRoundBreakdown {
                round: r.round,
                result: r
                    .golfers
                    .into_iter()
                    .find(|g| g.player_id == p)
                    .ok_or(Error::Invalid)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let mut result = GolferBreakdown {
        tournament_id: t,
        revision: String::new(),
        standing,
        rounds,
        finalized: b.overall.finalized,
    };
    result.revision = revision(&result)?;
    Ok(result)
}
pub async fn manager_breakdown(
    pool: &PgPool,
    session: Uuid,
    t: Uuid,
    u: Uuid,
) -> Result<ManagerBreakdown, Error> {
    let b = bundle(pool, session, t, Target::Manager(u)).await?;
    let standing = b
        .overall
        .managers
        .into_iter()
        .find(|s| s.id == u)
        .ok_or(AuthorizationError::NotFound)?;
    let rounds = b
        .rounds
        .into_iter()
        .map(|r| {
            Ok(ManagerRoundBreakdown {
                round: r.round,
                result: r
                    .managers
                    .into_iter()
                    .find(|m| m.user_id == u)
                    .ok_or(Error::Invalid)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let mut result = ManagerBreakdown {
        tournament_id: t,
        revision: String::new(),
        standing,
        rounds,
        finalized: b.overall.finalized,
    };
    result.revision = revision(&result)?;
    Ok(result)
}
