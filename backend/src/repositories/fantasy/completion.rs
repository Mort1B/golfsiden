use super::sources;
use crate::domain::{
    fantasy::projection::{Facts, unresolved_four_ball_inputs},
    round_completion::{CompletionIssue, CompletionIssueCode, RoundCompletionValidation},
    scorecards::ScoreOwner,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

// Caller owns either a coherent read snapshot or the parent round mutation lock.
// Do not use manager lineups or a client-side approximation of Fantasy rules.
pub(crate) async fn add_readiness(
    tx: &mut Transaction<'_, Postgres>,
    validation: &mut RoundCompletionValidation,
) -> Result<(), sqlx::Error> {
    let owners: Vec<_> = validation.owners.iter().map(|o| o.owner).collect();
    let issues = issues(tx, validation.round_id, &owners).await?;
    if !issues.is_empty() {
        validation.ready_to_lock = false;
    }
    validation.issues.extend(issues);
    Ok(())
}

pub(crate) async fn issues(
    tx: &mut Transaction<'_, Postgres>,
    round: Uuid,
    owners: &[ScoreOwner],
) -> Result<Vec<CompletionIssue>, sqlx::Error> {
    let enabled: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM rounds r JOIN fantasy_games f ON f.tournament_id=r.tournament_id WHERE r.id=$1 AND r.scoring_format='four_ball_stroke_play' AND r.status<>'draft' AND f.enabled)")
        .bind(round).fetch_one(&mut **tx).await?;
    if !enabled {
        return Ok(Vec::new());
    }
    let targets: Vec<_> = owners
        .iter()
        .map(|o| {
            let ScoreOwner::Team { id } = *o else {
                return Err(sqlx::Error::Protocol("invalid four-ball owner".into()));
            };
            Ok((round, "team".to_owned(), id))
        })
        .collect::<Result<_, _>>()?;
    let names: Vec<(Uuid, String)> = sqlx::query_as("SELECT p.id,p.display_name FROM players p JOIN team_memberships tm ON tm.player_id=p.id WHERE tm.round_id=$1 ORDER BY p.id")
        .bind(round).fetch_all(&mut **tx).await?;
    let mut issues = Vec::new();
    for source in sources::bulk_facts(tx, &targets).await? {
        let facts: Facts =
            serde_json::from_value(source.facts).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let missing = unresolved_four_ball_inputs(&facts)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        if missing.is_empty() {
            continue;
        }
        let details = missing
            .into_iter()
            .map(|(id, holes)| {
                let name = names
                    .iter()
                    .find(|(player, _)| *player == id)
                    .map(|(_, name)| name.as_str())
                    .ok_or_else(|| sqlx::Error::Protocol("missing partner identity".into()))?;
                Ok(format!(
                    "{}: hull {}",
                    name,
                    holes
                        .iter()
                        .map(i16::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()?
            .join("; ");
        issues.push(CompletionIssue {
            code: CompletionIssueCode::FantasyInputsUnresolved,
            message: details,
            owner: Some(ScoreOwner::Team {
                id: source.owner_id,
            }),
        });
    }
    Ok(issues)
}
