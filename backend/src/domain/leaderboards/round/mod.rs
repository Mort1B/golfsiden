mod owners;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::domain::{models::RoundStatus, scoring::hole_net_score};

use self::owners::{OwnerSeed, build_owner_seeds};
use super::{
    LeaderboardError, LeaderboardMember, LeaderboardMetric, LeaderboardOwner, RoundLeaderboard,
    RoundLeaderboardEntry, RoundLeaderboardFacts,
};

pub fn build_round_leaderboard(
    facts: &RoundLeaderboardFacts,
    metric: LeaderboardMetric,
) -> Result<RoundLeaderboard, LeaderboardError> {
    let round = &facts.round;
    if round.number_of_holes < 1 {
        return Err(LeaderboardError::InvalidStoredData);
    }
    if round.status == RoundStatus::Draft {
        if !facts.snapshots.is_empty()
            || !facts.scores.is_empty()
            || !facts.confirmations.is_empty()
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
        return Ok(RoundLeaderboard {
            round_id: round.round_id,
            tournament_id: round.tournament_id,
            status: round.status,
            scoring_format: round.scoring_format,
            metric,
            number_of_holes: round.number_of_holes as usize,
            entries: Vec::new(),
        });
    }

    let holes = validated_holes(facts)?;
    let snapshots = validated_snapshots(facts)?;
    let owners = build_owner_seeds(facts, &snapshots)?;
    let mut entries = assemble_entries(facts, &holes, owners)?;
    if entries.is_empty() {
        return Err(LeaderboardError::InvalidStoredData);
    }
    if matches!(round.status, RoundStatus::Completed | RoundStatus::Locked)
        && entries.iter().any(|entry| !entry.complete)
    {
        return Err(LeaderboardError::InvalidStoredData);
    }
    rank_entries(&mut entries, metric);
    Ok(RoundLeaderboard {
        round_id: round.round_id,
        tournament_id: round.tournament_id,
        status: round.status,
        scoring_format: round.scoring_format,
        metric,
        number_of_holes: round.number_of_holes as usize,
        entries,
    })
}

fn validated_holes(
    facts: &RoundLeaderboardFacts,
) -> Result<HashMap<uuid::Uuid, (i16, i16)>, LeaderboardError> {
    let expected = facts.round.number_of_holes;
    if facts.holes.len() != expected as usize {
        return Err(LeaderboardError::InvalidStoredData);
    }
    let mut by_id = HashMap::new();
    let mut numbers = HashSet::new();
    let mut indexes = HashSet::new();
    for hole in &facts.holes {
        if hole.round_id != facts.round.round_id
            || !(1..=expected).contains(&hole.hole_number)
            || !(1..=expected).contains(&hole.stroke_index)
            || by_id
                .insert(hole.hole_id, (hole.par, hole.stroke_index))
                .is_some()
            || !numbers.insert(hole.hole_number)
            || !indexes.insert(hole.stroke_index)
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
    }
    Ok(by_id)
}

fn validated_snapshots(
    facts: &RoundLeaderboardFacts,
) -> Result<HashMap<uuid::Uuid, &super::SnapshotFact>, LeaderboardError> {
    let mut snapshots = HashMap::new();
    for snapshot in &facts.snapshots {
        if snapshot.round_id != facts.round.round_id
            || snapshots.insert(snapshot.player_id, snapshot).is_some()
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
    }
    Ok(snapshots)
}

fn assemble_entries(
    facts: &RoundLeaderboardFacts,
    holes: &HashMap<uuid::Uuid, (i16, i16)>,
    owners: Vec<OwnerSeed<'_>>,
) -> Result<Vec<RoundLeaderboardEntry>, LeaderboardError> {
    let owner_set = owners
        .iter()
        .map(|owner| owner.owner)
        .collect::<HashSet<_>>();
    let mut scores: HashMap<LeaderboardOwner, Vec<&super::ScoreFact>> = HashMap::new();
    let mut score_keys = HashSet::new();
    for score in &facts.scores {
        let owner = score_owner(score.player_id, score.team_id)?;
        if score.round_id != facts.round.round_id
            || !owner_set.contains(&owner)
            || !holes.contains_key(&score.hole_id)
            || !score_keys.insert((owner, score.hole_id))
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
        scores.entry(owner).or_default().push(score);
    }
    let mut confirmed = HashSet::new();
    for confirmation in &facts.confirmations {
        let owner = score_owner(confirmation.player_id, confirmation.team_id)?;
        if confirmation.round_id != facts.round.round_id
            || !owner_set.contains(&owner)
            || !confirmed.insert(owner)
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
    }

    owners
        .into_iter()
        .map(|owner_seed| {
            let owner_scores = scores.remove(&owner_seed.owner).unwrap_or_default();
            let mut gross_total = 0;
            let mut net_total = 0;
            let mut par_played = 0;
            for score in &owner_scores {
                let (par, stroke_index) = holes
                    .get(&score.hole_id)
                    .ok_or(LeaderboardError::InvalidStoredData)?;
                gross_total += i32::from(score.gross_strokes);
                net_total += hole_net_score(
                    i32::from(score.gross_strokes),
                    owner_seed.playing_handicap,
                    i32::from(*stroke_index),
                    i32::from(facts.round.number_of_holes),
                )?;
                par_played += i32::from(*par);
            }
            let holes_scored = owner_scores.len();
            let complete = holes_scored == facts.round.number_of_holes as usize;
            if confirmed.contains(&owner_seed.owner) && !complete {
                return Err(LeaderboardError::InvalidStoredData);
            }
            Ok(RoundLeaderboardEntry {
                position: None,
                tied: false,
                owner: owner_seed.owner,
                owner_name: owner_seed.name.to_owned(),
                members: owner_seed.members,
                holes_scored,
                number_of_holes: facts.round.number_of_holes as usize,
                complete,
                confirmed: complete && confirmed.contains(&owner_seed.owner),
                playing_handicap: owner_seed.playing_handicap,
                gross_total,
                net_total,
                par_played,
                score_to_par: 0,
            })
        })
        .collect()
}

fn score_owner(
    player_id: Option<uuid::Uuid>,
    team_id: Option<uuid::Uuid>,
) -> Result<LeaderboardOwner, LeaderboardError> {
    match (player_id, team_id) {
        (Some(id), None) => Ok(LeaderboardOwner::Player { id }),
        (None, Some(id)) => Ok(LeaderboardOwner::Team { id }),
        _ => Err(LeaderboardError::InvalidStoredData),
    }
}

fn sort_members(members: &mut [LeaderboardMember]) {
    members.sort_by(|left, right| {
        left.display_order
            .is_none()
            .cmp(&right.display_order.is_none())
            .then(left.display_order.cmp(&right.display_order))
            .then_with(|| name_cmp(&left.display_name, &right.display_name))
            .then(left.player_id.cmp(&right.player_id))
    });
}

fn rank_entries(entries: &mut [RoundLeaderboardEntry], metric: LeaderboardMetric) {
    for entry in entries.iter_mut() {
        let selected = selected_total(entry, metric);
        entry.score_to_par = selected - entry.par_played;
    }
    entries.sort_by(
        |left, right| match (left.holes_scored > 0, right.holes_scored > 0) {
            (true, true) => left
                .score_to_par
                .cmp(&right.score_to_par)
                .then(right.holes_scored.cmp(&left.holes_scored))
                .then_with(|| name_cmp(&left.owner_name, &right.owner_name))
                .then(left.owner.id().cmp(&right.owner.id())),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => name_cmp(&left.owner_name, &right.owner_name)
                .then(left.owner.id().cmp(&right.owner.id())),
        },
    );
    let ranked = entries
        .iter()
        .take_while(|entry| entry.holes_scored > 0)
        .count();
    for index in 0..ranked {
        let tied_previous =
            index > 0 && entries[index - 1].score_to_par == entries[index].score_to_par;
        entries[index].position = Some(if tied_previous {
            entries[index - 1].position.unwrap_or(index)
        } else {
            index + 1
        });
        entries[index].tied = tied_previous
            || entries.get(index + 1).is_some_and(|next| {
                next.holes_scored > 0 && next.score_to_par == entries[index].score_to_par
            });
    }
}

fn selected_total(entry: &RoundLeaderboardEntry, metric: LeaderboardMetric) -> i32 {
    match metric {
        LeaderboardMetric::Gross => entry.gross_total,
        LeaderboardMetric::Net => entry.net_total,
    }
}

pub(crate) fn name_cmp(left: &str, right: &str) -> Ordering {
    left.to_lowercase().cmp(&right.to_lowercase())
}
