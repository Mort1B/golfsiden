use crate::domain::{
    leaderboards::{LeaderboardMetric, TournamentLeaderboard},
    models::TournamentTieBreakPolicy,
    score_visibility::VisibilityMetadata,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct PublicResults {
    pub tournament_name: String,
    pub metric: LeaderboardMetric,
    pub required_counted_rounds: usize,
    pub final_round_number: i16,
    pub tie_break_policy: TournamentTieBreakPolicy,
    pub visibility: VisibilityMetadata,
    pub entries: Vec<PublicStanding>,
}
pub struct PublicStanding {
    pub equivalent_basis: bool,
    pub position: Option<usize>,
    pub tied: bool,
    pub display_name: String,
    pub completed_rounds: usize,
    pub counted_contributions: usize,
    pub eligible: bool,
    pub total: i32,
    pub par_total: i32,
    pub score_to_par: i32,
    pub provisional: bool,
    pub provisional_holes_scored: usize,
    pub tie_break_score_to_par: Option<i32>,
}

/// Input must already have been assembled with the ordinary non-admin visibility
/// policy. This allowlist removes identities and private detail, never derives
/// sporting ranks from unprojected data.
pub fn project(name: String, board: TournamentLeaderboard) -> PublicResults {
    let metric = board.metric;
    PublicResults {
        tournament_name: name,
        metric,
        required_counted_rounds: board.required_counted_rounds,
        final_round_number: board.final_round_number,
        tie_break_policy: board.tie_break_policy,
        visibility: board.visibility,
        entries: board
            .entries
            .into_iter()
            .map(|entry| {
                let provisional_holes_scored = entry
                    .contributions
                    .iter()
                    .filter(|c| c.counted && c.provisional)
                    .map(|c| c.holes_scored)
                    .sum();
                PublicStanding {
                    equivalent_basis: entry.equivalents.is_some(),
                    position: entry.position,
                    tied: entry.tied,
                    display_name: entry.display_name,
                    completed_rounds: entry.completed_rounds,
                    counted_contributions: entry.counted_contributions,
                    eligible: entry.eligible,
                    total: match metric {
                        LeaderboardMetric::Gross => entry.gross_total,
                        LeaderboardMetric::Net => entry.net_total,
                    },
                    par_total: entry.par_total,
                    score_to_par: entry.score_to_par,
                    provisional: provisional_holes_scored > 0,
                    provisional_holes_scored,
                    tie_break_score_to_par: entry.tie_break_score_to_par,
                }
            })
            .collect(),
    }
}

impl Serialize for PublicStanding {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        macro_rules! fields {($($field:ident),+)=>{$(map.serialize_entry(stringify!($field),&self.$field)?;)+};}
        fields!(
            position,
            tied,
            display_name,
            completed_rounds,
            counted_contributions,
            eligible,
            provisional,
            provisional_holes_scored
        );
        if self.equivalent_basis {
            map.serialize_entry("value",&serde_json::json!({"type":"overall_equivalent","version":1,"selected":self.score_to_par,"tie_break":self.tie_break_score_to_par}))?;
        } else {
            fields!(total, par_total, score_to_par, tie_break_score_to_par);
        }
        map.end()
    }
}
