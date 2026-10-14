use super::*;
use crate::domain::{
    round_awards::{AwardFact, summary},
    score_visibility::VisibilityMode,
};

pub(super) fn adjust_round(board: &mut RoundLeaderboard, awards: &[AwardFact]) {
    if board.visibility.mode != VisibilityMode::Full || board.metric != LeaderboardMetric::Net {
        return;
    }
    for entry in &mut board.entries {
        match entry.owner {
            LeaderboardOwner::Player { id } => {
                entry.net_awards = summary(awards, id, entry.net_total, entry.stableford.as_ref());
                let adjustment = entry.net_awards.as_ref().map_or(0, |a| a.adjustment());
                if entry.stableford.is_none() {
                    entry.net_total += adjustment;
                    entry.score_to_par += adjustment;
                }
            }
            LeaderboardOwner::Team { .. } => {
                for member in &mut entry.members {
                    member.net_awards = summary(awards, member.player_id, entry.net_total, None);
                }
            }
        }
    }
    if awards.is_empty() {
        return;
    }
    let key = |e: &RoundLeaderboardEntry| {
        e.stableford.as_ref().map_or(e.score_to_par, |v| {
            v.net_equivalent + e.net_awards.as_ref().map_or(0, |a| a.adjustment())
        })
    };
    board.entries.sort_by(|a, b| {
        (b.holes_scored > 0)
            .cmp(&(a.holes_scored > 0))
            .then_with(|| key(a).cmp(&key(b)))
            .then(b.holes_scored.cmp(&a.holes_scored))
            .then_with(|| round::name_cmp(&a.owner_name, &b.owner_name))
            .then(a.owner.id().cmp(&b.owner.id()))
    });
    let mut offset = 0;
    for group in board
        .entries
        .chunk_by_mut(|a, b| (a.holes_scored > 0, key(a)) == (b.holes_scored > 0, key(b)))
    {
        let tied = group.len() > 1;
        for e in group.iter_mut().filter(|e| e.holes_scored > 0) {
            e.position = Some(offset + 1);
            e.tied = tied;
        }
        offset += group.len();
    }
}
