use std::cmp::Ordering;

use super::super::{LeaderboardMetric, TournamentContribution, TournamentLeaderboardEntry};

pub(super) fn select_best(
    contributions: &mut [CandidateContribution],
    required: usize,
    metric: LeaderboardMetric,
) {
    contributions.sort_by(|left, right| {
        selected_score(left, metric)
            .cmp(&selected_score(right, metric))
            .then(left.round_number.cmp(&right.round_number))
            .then(left.value.round_id.cmp(&right.value.round_id))
    });
    for candidate in contributions.iter_mut().take(required) {
        candidate.value.counted = true;
    }
    contributions.sort_by(|left, right| {
        left.round_number
            .cmp(&right.round_number)
            .then(left.value.round_id.cmp(&right.value.round_id))
    });
}

pub(super) fn rank_entries(entries: &mut [TournamentLeaderboardEntry]) {
    entries.sort_by(
        |left, right| match (left.completed_rounds > 0, right.completed_rounds > 0) {
            (true, true) => right
                .counted_contributions
                .cmp(&left.counted_contributions)
                .then(left.score_to_par.cmp(&right.score_to_par))
                .then_with(|| {
                    super::super::round::name_cmp(&left.display_name, &right.display_name)
                })
                .then(left.player_id.cmp(&right.player_id)),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => {
                super::super::round::name_cmp(&left.display_name, &right.display_name)
                    .then(left.player_id.cmp(&right.player_id))
            }
        },
    );

    let ranked = entries
        .iter()
        .take_while(|entry| entry.completed_rounds > 0)
        .count();
    for index in 0..ranked {
        let tied_previous = index > 0 && rank_key(&entries[index - 1]) == rank_key(&entries[index]);
        entries[index].position = Some(if tied_previous {
            entries[index - 1].position.unwrap_or(index)
        } else {
            index + 1
        });
        entries[index].tied = tied_previous
            || entries
                .get(index + 1)
                .is_some_and(|next| rank_key(next) == rank_key(&entries[index]));
    }
}

#[derive(Debug, Clone)]
pub(super) struct CandidateContribution {
    pub round_number: i16,
    pub value: TournamentContribution,
}

fn selected_score(candidate: &CandidateContribution, metric: LeaderboardMetric) -> i32 {
    match metric {
        LeaderboardMetric::Gross => candidate.value.gross_total - candidate.value.par_total,
        LeaderboardMetric::Net => candidate.value.net_total - candidate.value.par_total,
    }
}

fn rank_key(entry: &TournamentLeaderboardEntry) -> (usize, i32) {
    (entry.counted_contributions, entry.score_to_par)
}
