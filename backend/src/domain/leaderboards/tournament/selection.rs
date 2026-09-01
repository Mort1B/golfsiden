use std::cmp::Ordering;

use super::super::{LeaderboardMetric, TournamentContribution, TournamentLeaderboardEntry};

pub(super) fn select_best(
    contributions: &mut [CandidateContribution],
    required: usize,
    mandatory_round_id: Option<uuid::Uuid>,
    metric: LeaderboardMetric,
) {
    contributions.sort_by(|left, right| {
        selected_score(left, metric)
            .cmp(&selected_score(right, metric))
            .then(left.round_number.cmp(&right.round_number))
            .then(left.value.round_id.cmp(&right.value.round_id))
    });
    let optional_slots = if let Some(mandatory_round_id) = mandatory_round_id {
        if let Some(mandatory) = contributions
            .iter_mut()
            .find(|candidate| candidate.value.round_id == mandatory_round_id)
        {
            mandatory.value.counted = true;
        }
        required.saturating_sub(1)
    } else {
        required
    };
    for candidate in contributions
        .iter_mut()
        .filter(|candidate| !candidate.value.mandatory)
        .take(optional_slots)
    {
        candidate.value.counted = true;
    }
    contributions.sort_by(|left, right| {
        left.round_number
            .cmp(&right.round_number)
            .then(left.value.round_id.cmp(&right.value.round_id))
    });
}

pub(super) fn rank_entries(entries: &mut [TournamentLeaderboardEntry]) {
    entries.sort_by(|left, right| {
        match (
            left.counted_contributions > 0,
            right.counted_contributions > 0,
        ) {
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
        }
    });

    let ranked = entries
        .iter()
        .take_while(|entry| entry.counted_contributions > 0)
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
