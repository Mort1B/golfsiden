use std::cmp::Ordering;

use super::super::{LeaderboardMetric, TournamentLeaderboardEntry};
use super::contributions::CandidateContribution;

pub(super) struct Qualification {
    pub counted: usize,
    pub eligible: bool,
}

pub(super) fn completed_qualification(
    contributions: &[CandidateContribution],
    required: usize,
    mandatory_round_id: Option<uuid::Uuid>,
    metric: LeaderboardMetric,
) -> Qualification {
    let mut completed = contributions
        .iter()
        .filter(|candidate| !candidate.value.provisional)
        .collect::<Vec<_>>();
    sort_candidates(&mut completed, metric);
    let mandatory_completed = mandatory_round_id.is_none_or(|mandatory_id| {
        completed
            .iter()
            .any(|candidate| candidate.value.round_id == mandatory_id)
    });
    let mandatory_count = usize::from(mandatory_round_id.is_some() && mandatory_completed);
    let optional_slots = if mandatory_round_id.is_some() {
        required.saturating_sub(1)
    } else {
        required
    };
    let optional_count = completed
        .iter()
        .filter(|candidate| !candidate.value.mandatory)
        .take(optional_slots)
        .count();
    let counted = mandatory_count + optional_count;
    Qualification {
        counted,
        eligible: counted == required && mandatory_completed,
    }
}

pub(super) fn select_displayed(
    contributions: &mut [CandidateContribution],
    required: usize,
    mandatory_round_id: Option<uuid::Uuid>,
    metric: LeaderboardMetric,
) {
    contributions.sort_by(|left, right| candidate_cmp(left, right, metric));
    if let Some(mandatory_round_id) = mandatory_round_id
        && let Some(mandatory) = contributions
            .iter_mut()
            .find(|candidate| candidate.value.round_id == mandatory_round_id)
    {
        mandatory.value.counted = true;
    }
    let optional_slots = if mandatory_round_id.is_some() {
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
    entries.sort_by(
        |left, right| match (has_selected(left), has_selected(right)) {
            (true, true) => right
                .counted_contributions
                .cmp(&left.counted_contributions)
                .then(left.score_to_par.cmp(&right.score_to_par))
                .then_with(|| provisional_progress(right).cmp(&provisional_progress(left)))
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
        .take_while(|entry| has_selected(entry))
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

fn sort_candidates(candidates: &mut [&CandidateContribution], metric: LeaderboardMetric) {
    candidates.sort_by(|left, right| candidate_cmp(left, right, metric));
}

fn candidate_cmp(
    left: &CandidateContribution,
    right: &CandidateContribution,
    metric: LeaderboardMetric,
) -> Ordering {
    selected_score(left, metric)
        .cmp(&selected_score(right, metric))
        .then(left.round_number.cmp(&right.round_number))
        .then(left.value.round_id.cmp(&right.value.round_id))
}

fn selected_score(candidate: &CandidateContribution, metric: LeaderboardMetric) -> i32 {
    match metric {
        LeaderboardMetric::Gross => candidate.value.gross_total - candidate.value.par_total,
        LeaderboardMetric::Net => candidate.value.net_total - candidate.value.par_total,
    }
}

fn has_selected(entry: &TournamentLeaderboardEntry) -> bool {
    entry
        .contributions
        .iter()
        .any(|contribution| contribution.counted)
}

fn provisional_progress(entry: &TournamentLeaderboardEntry) -> usize {
    entry
        .contributions
        .iter()
        .filter(|contribution| contribution.counted && contribution.provisional)
        .map(|contribution| contribution.holes_scored)
        .sum()
}

fn rank_key(entry: &TournamentLeaderboardEntry) -> (usize, i32) {
    (entry.counted_contributions, entry.score_to_par)
}
