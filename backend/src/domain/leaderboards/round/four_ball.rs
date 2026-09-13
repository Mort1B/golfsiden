//! Derived side totals consume numeric player facts, never synthetic team scores.
use super::*;
use crate::domain::{
    four_ball::{GrossScore, PlayerHole, PlayerHoleInput, aggregate_hole},
    leaderboards::SnapshotFact,
};
use uuid::Uuid;
pub(super) fn build(
    facts: &RoundLeaderboardFacts,
    metric: LeaderboardMetric,
    visibility: VisibilityMetadata,
    holes: &HashMap<Uuid, (i16, i16)>,
    snapshots: &HashMap<Uuid, &SnapshotFact>,
) -> Result<RoundLeaderboard, LeaderboardError> {
    let invalid = || LeaderboardError::InvalidStoredData;
    if facts.round.number_of_holes != 18 {
        return Err(invalid());
    }
    let mut team_members = HashMap::<Uuid, Vec<LeaderboardMember>>::new();
    let mut assigned = HashSet::new();
    let team_ids = facts
        .teams
        .iter()
        .map(|t| t.team_id)
        .collect::<HashSet<_>>();
    if team_ids.len() != facts.teams.len() {
        return Err(invalid());
    }
    for member in &facts.memberships {
        if member.round_id != facts.round.round_id
            || !team_ids.contains(&member.team_id)
            || !snapshots.contains_key(&member.player_id)
            || !assigned.insert(member.player_id)
        {
            return Err(invalid());
        }
        team_members
            .entry(member.team_id)
            .or_default()
            .push(LeaderboardMember {
                player_id: member.player_id,
                display_name: member.display_name.clone(),
                display_order: member.display_order,
            });
    }
    if assigned.len() != snapshots.len() {
        return Err(invalid());
    }
    let mut scores = HashMap::new();
    for score in &facts.scores {
        let player = score.player_id.ok_or_else(invalid)?;
        if score.round_id != facts.round.round_id
            || score.team_id.is_some()
            || !assigned.contains(&player)
            || !holes.contains_key(&score.hole_id)
            || scores
                .insert((player, score.hole_id), score.gross_strokes)
                .is_some()
        {
            return Err(invalid());
        }
        GrossScore::new(i32::from(score.gross_strokes)).map_err(|_| invalid())?;
    }
    let mut confirmations = HashSet::new();
    for confirmation in &facts.confirmations {
        let team = confirmation.team_id.ok_or_else(invalid)?;
        if confirmation.round_id != facts.round.round_id
            || confirmation.player_id.is_some()
            || !team_ids.contains(&team)
            || !confirmations.insert(team)
        {
            return Err(invalid());
        }
    }
    let restricted = visibility.mode == VisibilityMode::FrontNine;
    let mut entries = Vec::new();
    for team in &facts.teams {
        if team.round_id != facts.round.round_id {
            return Err(invalid());
        }
        let mut members = team_members.remove(&team.team_id).ok_or_else(invalid)?;
        sort_members(&mut members);
        let [a, b] = members.as_slice() else {
            return Err(invalid());
        };
        let partners = [a, b].map(|m| snapshots.get(&m.player_id).copied().ok_or_else(invalid));
        let [a, b] = partners;
        let partners = [a?, b?];
        let mut entry = RoundLeaderboardEntry {
            stableford: None,
            position: None,
            tied: false,
            owner: LeaderboardOwner::Team { id: team.team_id },
            owner_name: team.team_name.clone(),
            members: Vec::new(),
            holes_scored: 0,
            number_of_holes: 18,
            complete: None,
            confirmed: None,
            playing_handicap: None,
            gross_total: 0,
            net_total: 0,
            par_played: 0,
            score_to_par: 0,
        };
        for hole in facts
            .holes
            .iter()
            .filter(|h| !restricted || h.hole_number <= 9)
        {
            let inputs: [Result<PlayerHole, LeaderboardError>; 2] = partners.map(|p| {
                let input = scores
                    .get(&(p.player_id, hole.hole_id))
                    .map(|gross| GrossScore::new(i32::from(*gross)).map(PlayerHoleInput::Numeric))
                    .transpose()
                    .map_err(|_| invalid())?
                    .unwrap_or(PlayerHoleInput::Unentered);
                Ok(PlayerHole {
                    player_id: p.player_id,
                    playing_handicap: if facts.round.handicap_enabled {
                        p.playing_handicap
                    } else {
                        0
                    },
                    input,
                })
            });
            let [a, b] = inputs;
            if let Some(result) = aggregate_hole(
                [a?, b?],
                u8::try_from(hole.stroke_index).map_err(|_| invalid())?,
            )
            .map_err(|_| invalid())?
            {
                entry.holes_scored += 1;
                entry.gross_total += result.gross.score;
                entry.net_total += result.net.score;
                entry.par_played += i32::from(hole.par);
            }
        }
        if !restricted {
            entry.complete = Some(entry.holes_scored == 18);
            entry.confirmed = Some(confirmations.contains(&team.team_id));
            if (entry.confirmed == Some(true) || facts.round.status == RoundStatus::Locked)
                && entry.complete != Some(true)
            {
                return Err(invalid());
            }
        }
        entry.members = members;
        entries.push(entry);
    }
    if entries.is_empty() {
        return Err(invalid());
    }
    rank_entries(&mut entries, metric);
    Ok(RoundLeaderboard {
        round_id: facts.round.round_id,
        tournament_id: facts.round.tournament_id,
        status: facts.round.status,
        scoring_format: facts.round.scoring_format,
        metric,
        number_of_holes: 18,
        visible_hole_count: if restricted { 9 } else { 18 },
        visibility,
        entries,
    })
}
