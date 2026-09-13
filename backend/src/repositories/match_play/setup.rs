use super::*;
use crate::domain::models::{PairingValidation, ReadinessIssue, ReadinessIssueCode, Round};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pair {
    pub first_player_id: Uuid,
    pub second_player_id: Uuid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignments {
    pub expected_round_updated_at: DateTime<Utc>,
    pub matches: Vec<Pair>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub expected_round_updated_at: DateTime<Utc>,
    pub handicap_enabled: bool,
}
async fn draft(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    round: Uuid,
    expected: DateTime<Utc>,
) -> Result<Context, Error> {
    let c = context(tx, round, true).await?;
    let (actor, role) = member(tx, session, c.tournament_id).await?;
    if role != TournamentRole::Admin {
        return Err(Error::Forbidden);
    }
    if c.status != RoundStatus::Draft {
        return Err(Error::Conflict("round_not_draft"));
    }
    let current =
        sqlx::query_scalar::<_, DateTime<Utc>>("SELECT updated_at FROM rounds WHERE id=$1")
            .bind(round)
            .fetch_one(&mut **tx)
            .await?;
    if expected != current {
        return Err(Error::Conflict("round_configuration_stale"));
    }
    write_context(tx, session, actor, round).await?;
    Ok(c)
}
pub async fn replace(
    pool: &PgPool,
    session: Uuid,
    round: Uuid,
    input: Assignments,
) -> Result<Uuid, Error> {
    if input.matches.len() > 500 {
        return Err(Error::Invalid("too many matches"));
    }
    let mut tx = pool.begin().await?;
    let c = draft(&mut tx, session, round, input.expected_round_updated_at).await?;
    let mut seen = std::collections::HashSet::new();
    for p in &input.matches {
        if !seen.insert(p.first_player_id) || !seen.insert(p.second_player_id) {
            return Err(Error::Invalid(
                "each opponent may appear in exactly one match",
            ));
        }
        let valid=sqlx::query_scalar::<_,bool>("SELECT count(*)=2 FROM tournament_players tp JOIN players p ON p.id=tp.player_id WHERE tp.tournament_id=$1 AND tp.player_id=ANY($2) AND tp.status='active' AND p.active").bind(c.tournament_id).bind(vec![p.first_player_id,p.second_player_id]).fetch_one(&mut *tx).await?;
        if !valid {
            return Err(Error::Invalid("opponents must be distinct active entrants"));
        }
        let same=sqlx::query_scalar::<_,bool>("SELECT count(*)=2 AND count(DISTINCT flight_id)=1 FROM flight_memberships WHERE round_id=$1 AND player_id=ANY($2)").bind(round).bind(vec![p.first_player_id,p.second_player_id]).fetch_one(&mut *tx).await?;
        if !same {
            return Err(Error::Invalid(
                "opponents must be assigned to the same flight",
            ));
        }
    }
    let old=sqlx::query_as::<_,(Uuid,Uuid)>("SELECT first_player_id,second_player_id FROM singles_matches WHERE round_id=$1 ORDER BY first_player_id,second_player_id").bind(round).fetch_all(&mut *tx).await?;
    let mut next = input
        .matches
        .iter()
        .map(|p| (p.first_player_id, p.second_player_id))
        .collect::<Vec<_>>();
    next.sort();
    if old != next {
        sqlx::query("DELETE FROM singles_matches WHERE round_id=$1")
            .bind(round)
            .execute(&mut *tx)
            .await?;
        for (first, second) in next {
            sqlx::query("INSERT INTO singles_matches(id,round_id,tournament_id,first_player_id,second_player_id) VALUES($1,$2,$3,$4,$5)").bind(Uuid::new_v4()).bind(round).bind(c.tournament_id).bind(first).bind(second).execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE rounds SET updated_at=clock_timestamp() WHERE id=$1")
            .bind(round)
            .execute(&mut *tx)
            .await?;
    }
    live(&mut tx, session).await?;
    tx.commit().await?;
    Ok(c.tournament_id)
}
pub async fn settings(
    pool: &PgPool,
    session: Uuid,
    round: Uuid,
    input: Settings,
) -> Result<Round, Error> {
    let mut tx = pool.begin().await?;
    draft(&mut tx, session, round, input.expected_round_updated_at).await?;
    sqlx::query("UPDATE rounds SET handicap_enabled=$2,handicap_allowance_percent=100 WHERE id=$1 AND handicap_enabled IS DISTINCT FROM $2").bind(round).bind(input.handicap_enabled).execute(&mut *tx).await?;
    let r = sqlx::query_as::<_, Round>("SELECT * FROM rounds WHERE id=$1")
        .bind(round)
        .fetch_one(&mut *tx)
        .await?;
    live(&mut tx, session).await?;
    tx.commit().await?;
    Ok(r)
}
pub async fn add_readiness(
    connection: &mut sqlx::PgConnection,
    validation: &mut PairingValidation,
) -> Result<(), sqlx::Error> {
    let format =
        sqlx::query_scalar::<_, ScoringFormat>("SELECT scoring_format FROM rounds WHERE id=$1")
            .bind(validation.round_id)
            .fetch_optional(&mut *connection)
            .await?;
    if format != Some(ScoringFormat::SinglesMatchPlay) {
        return Ok(());
    }
    let ready = sqlx::query_scalar::<_, bool>("SELECT singles_assignments_ready($1)")
        .bind(validation.round_id)
        .fetch_one(connection)
        .await?;
    if !ready {
        validation.ready = false;
        validation.issues.push(ReadinessIssue{code:ReadinessIssueCode::InvalidMatchAssignments,message:"every active entrant requires one distinct opponent in the same flight, starting at hole 1"});
    }
    Ok(())
}
