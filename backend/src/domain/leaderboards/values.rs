//! Versioned transport keeps legacy stroke contracts intact and gives equivalents explicit units.
use super::types::*;
use serde::{Serialize, Serializer, ser::SerializeMap};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EquivalentTotals {
    pub gross: i32,
    pub net: i32,
}
impl TournamentContribution {
    pub fn equivalent(&self, metric: LeaderboardMetric) -> i32 {
        match (&self.stableford, metric) {
            (Some(v), LeaderboardMetric::Gross) => v.gross_equivalent,
            (Some(v), LeaderboardMetric::Net) => v.net_equivalent,
            (None, LeaderboardMetric::Gross) => self.gross_total - self.par_total,
            (None, LeaderboardMetric::Net) => self.net_total - self.par_total,
        }
    }
}
macro_rules! fields { ($map:ident,$self:ident,$($field:ident),+)=>{$($map.serialize_entry(stringify!($field),&$self.$field)?;)+}; }
impl Serialize for RoundLeaderboardEntry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        fields!(
            map,
            self,
            position,
            tied,
            owner,
            owner_name,
            members,
            holes_scored,
            number_of_holes,
            complete,
            confirmed,
            playing_handicap
        );
        if let Some(values) = &self.stableford {
            map.serialize_entry("value",&serde_json::json!({"type":"stableford","version":1,"gross_points":values.gross_points,"net_points":values.net_points,"gross_equivalent":values.gross_equivalent,"net_equivalent":values.net_equivalent,"actual_gross_total":values.actual_gross_total,"actual_net_total":values.actual_net_total}))?;
        } else {
            fields!(map, self, gross_total, net_total, par_played, score_to_par);
        }
        map.end()
    }
}
impl Serialize for TournamentContribution {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        fields!(
            map,
            self,
            round_id,
            mandatory,
            provisional,
            owner,
            owner_name,
            holes_scored,
            number_of_holes,
            counted
        );
        if let Some(values) = &self.stableford {
            map.serialize_entry("value",&serde_json::json!({"type":"stableford","version":1,"gross_points":values.gross_points,"net_points":values.net_points,"gross_equivalent":values.gross_equivalent,"net_equivalent":values.net_equivalent,"actual_gross_total":values.actual_gross_total,"actual_net_total":values.actual_net_total}))?;
        } else {
            fields!(map, self, gross_total, net_total, par_total, score_to_par);
        }
        map.end()
    }
}
impl Serialize for TournamentLeaderboardEntry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        fields!(
            map,
            self,
            position,
            tied,
            player_id,
            display_name,
            status,
            completed_rounds,
            counted_contributions,
            eligible,
            contributions,
            current_team
        );
        if let Some(values) = &self.equivalents {
            map.serialize_entry("value",&serde_json::json!({"type":"overall_equivalent","version":1,"gross":values.gross,"net":values.net,"selected":self.score_to_par,"tie_break":self.tie_break_score_to_par}))?;
        } else {
            fields!(
                map,
                self,
                gross_total,
                net_total,
                par_total,
                score_to_par,
                tie_break_score_to_par
            );
        }
        map.end()
    }
}
