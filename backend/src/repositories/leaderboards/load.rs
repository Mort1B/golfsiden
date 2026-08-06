use std::collections::HashMap;

use sqlx::PgConnection;
use uuid::Uuid;

use crate::domain::leaderboards::{
    ConfirmationFact, HoleFact, MembershipFact, RoundFact, RoundLeaderboardFacts, ScoreFact,
    SnapshotFact, TeamFact,
};

use super::rows::{ConfirmationRow, HoleRow, MembershipRow, ScoreRow, SnapshotRow, TeamRow};

pub(super) async fn related(
    connection: &mut PgConnection,
    rounds: Vec<RoundFact>,
) -> Result<Vec<RoundLeaderboardFacts>, sqlx::Error> {
    let round_ids = rounds
        .iter()
        .map(|round| round.round_id)
        .collect::<Vec<_>>();
    if round_ids.is_empty() {
        return Ok(Vec::new());
    }
    let holes = sqlx::query_as::<_, HoleRow>(
        "SELECT r.id AS round_id, h.id AS hole_id, h.hole_number, h.par, h.stroke_index FROM rounds r JOIN holes h ON h.tee_id = r.tee_id WHERE r.id = ANY($1) ORDER BY r.id, h.hole_number, h.id",
    )
    .bind(&round_ids)
    .fetch_all(&mut *connection)
    .await?;
    let snapshots = sqlx::query_as::<_, SnapshotRow>(
        "SELECT rhs.round_id, rhs.player_id, p.display_name, rhs.course_handicap, rhs.playing_handicap FROM round_handicap_snapshots rhs JOIN players p ON p.id = rhs.player_id WHERE rhs.round_id = ANY($1) ORDER BY rhs.round_id, rhs.player_id",
    )
    .bind(&round_ids)
    .fetch_all(&mut *connection)
    .await?;
    let teams = sqlx::query_as::<_, TeamRow>(
        "SELECT round_id, id AS team_id, name AS team_name FROM teams WHERE round_id = ANY($1) ORDER BY round_id, id",
    )
    .bind(&round_ids)
    .fetch_all(&mut *connection)
    .await?;
    let memberships = sqlx::query_as::<_, MembershipRow>(
        "SELECT tm.round_id, tm.team_id, tm.player_id, p.display_name, tm.display_order FROM team_memberships tm JOIN players p ON p.id = tm.player_id WHERE tm.round_id = ANY($1) ORDER BY tm.round_id, tm.team_id, tm.player_id",
    )
    .bind(&round_ids)
    .fetch_all(&mut *connection)
    .await?;
    let scores = sqlx::query_as::<_, ScoreRow>(
        "SELECT round_id, hole_id, player_id, team_id, gross_strokes FROM scores WHERE round_id = ANY($1) ORDER BY round_id, hole_id, player_id, team_id",
    )
    .bind(&round_ids)
    .fetch_all(&mut *connection)
    .await?;
    let confirmations = sqlx::query_as::<_, ConfirmationRow>(
        "SELECT round_id, player_id, team_id FROM scorecard_confirmations WHERE round_id = ANY($1) ORDER BY round_id, player_id, team_id",
    )
    .bind(&round_ids)
    .fetch_all(&mut *connection)
    .await?;

    let mut related = Related::default();
    for row in holes {
        related
            .holes
            .entry(row.round_id)
            .or_default()
            .push(HoleFact {
                round_id: row.round_id,
                hole_id: row.hole_id,
                hole_number: row.hole_number,
                par: row.par,
                stroke_index: row.stroke_index,
            });
    }
    for row in snapshots {
        related
            .snapshots
            .entry(row.round_id)
            .or_default()
            .push(SnapshotFact {
                round_id: row.round_id,
                player_id: row.player_id,
                display_name: row.display_name,
                course_handicap: row.course_handicap,
                playing_handicap: row.playing_handicap,
            });
    }
    for row in teams {
        related
            .teams
            .entry(row.round_id)
            .or_default()
            .push(TeamFact {
                round_id: row.round_id,
                team_id: row.team_id,
                team_name: row.team_name,
            });
    }
    for row in memberships {
        related
            .memberships
            .entry(row.round_id)
            .or_default()
            .push(MembershipFact {
                round_id: row.round_id,
                team_id: row.team_id,
                player_id: row.player_id,
                display_name: row.display_name,
                display_order: row.display_order,
            });
    }
    for row in scores {
        related
            .scores
            .entry(row.round_id)
            .or_default()
            .push(ScoreFact {
                round_id: row.round_id,
                hole_id: row.hole_id,
                player_id: row.player_id,
                team_id: row.team_id,
                gross_strokes: row.gross_strokes,
            });
    }
    for row in confirmations {
        related
            .confirmations
            .entry(row.round_id)
            .or_default()
            .push(ConfirmationFact {
                round_id: row.round_id,
                player_id: row.player_id,
                team_id: row.team_id,
            });
    }
    Ok(rounds
        .into_iter()
        .map(|round| related.take(round))
        .collect())
}

#[derive(Default)]
struct Related {
    holes: HashMap<Uuid, Vec<HoleFact>>,
    snapshots: HashMap<Uuid, Vec<SnapshotFact>>,
    teams: HashMap<Uuid, Vec<TeamFact>>,
    memberships: HashMap<Uuid, Vec<MembershipFact>>,
    scores: HashMap<Uuid, Vec<ScoreFact>>,
    confirmations: HashMap<Uuid, Vec<ConfirmationFact>>,
}

impl Related {
    fn take(&mut self, round: RoundFact) -> RoundLeaderboardFacts {
        let id = round.round_id;
        RoundLeaderboardFacts {
            round,
            holes: self.holes.remove(&id).unwrap_or_default(),
            snapshots: self.snapshots.remove(&id).unwrap_or_default(),
            teams: self.teams.remove(&id).unwrap_or_default(),
            memberships: self.memberships.remove(&id).unwrap_or_default(),
            scores: self.scores.remove(&id).unwrap_or_default(),
            confirmations: self.confirmations.remove(&id).unwrap_or_default(),
        }
    }
}
