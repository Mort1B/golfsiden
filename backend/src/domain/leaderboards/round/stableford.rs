use super::*;
use crate::domain::{
    leaderboards::SnapshotFact,
    stableford::{self, HoleLayout, PlayerHoleInput, card::Values},
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
    if facts.round.number_of_holes != 18 || !facts.scores.is_empty() || snapshots.is_empty() {
        return Err(invalid());
    }
    let restricted = visibility.mode == VisibilityMode::FrontNine;
    let mut layout = [HoleLayout {
        par: 4,
        stroke_index: 1,
    }; 18];
    let mut visible = [true; 18];
    let mut ordered = facts.holes.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|h| h.hole_number);
    for ((hole, item), mask) in ordered.iter().zip(&mut layout).zip(&mut visible) {
        *item = HoleLayout {
            par: u8::try_from(hole.par).map_err(|_| invalid())?,
            stroke_index: u8::try_from(hole.stroke_index).map_err(|_| invalid())?,
        };
        *mask = !restricted || hole.hole_number <= 9;
    }
    let mut inputs = HashMap::new();
    for input in &facts.stableford_inputs {
        if input.round_id != facts.round.round_id
            || !snapshots.contains_key(&input.player_id)
            || !holes.contains_key(&input.hole_id)
            || input.input == PlayerHoleInput::Unentered
            || inputs
                .insert((input.player_id, input.hole_id), input.input)
                .is_some()
        {
            return Err(invalid());
        }
    }
    let mut confirmed = HashSet::new();
    for c in &facts.confirmations {
        let player = c.player_id.ok_or_else(invalid)?;
        if c.round_id != facts.round.round_id
            || c.team_id.is_some()
            || !snapshots.contains_key(&player)
            || !confirmed.insert(player)
        {
            return Err(invalid());
        }
    }
    let mut entries = Vec::new();
    for snapshot in snapshots.values() {
        let mut player_inputs = [PlayerHoleInput::Unentered; 18];
        for (hole, input) in ordered.iter().zip(&mut player_inputs) {
            *input = inputs
                .get(&(snapshot.player_id, hole.hole_id))
                .copied()
                .unwrap_or(PlayerHoleInput::Unentered);
        }
        let handicap = if facts.round.handicap_enabled {
            snapshot.playing_handicap
        } else {
            0
        };
        let card = stableford::project_card(player_inputs, layout, handicap, visible)
            .map_err(|_| invalid())?;
        let values = card
            .totals
            .map(|t| Values {
                gross_points: t.points.gross.value(),
                net_points: t.points.net.value(),
                gross_equivalent: t.overall.gross.value(),
                net_equivalent: t.overall.net.value(),
                actual_gross_total: card.full_stroke_totals.map(|s| s.gross.value()),
                actual_net_total: card.full_stroke_totals.map(|s| s.net.value()),
            })
            .unwrap_or(Values {
                gross_points: 0,
                net_points: 0,
                gross_equivalent: 0,
                net_equivalent: 0,
                actual_gross_total: None,
                actual_net_total: None,
            });
        if !restricted && confirmed.contains(&snapshot.player_id) && !card.is_complete() {
            return Err(invalid());
        }
        entries.push(RoundLeaderboardEntry {
            stableford: Some(values),
            position: None,
            tied: false,
            owner: LeaderboardOwner::Player {
                id: snapshot.player_id,
            },
            owner_name: snapshot.display_name.clone(),
            members: Vec::new(),
            holes_scored: usize::from(card.resolved_holes),
            number_of_holes: 18,
            complete: (!restricted).then_some(card.is_complete()),
            confirmed: (!restricted).then_some(confirmed.contains(&snapshot.player_id)),
            playing_handicap: Some(i32::from(handicap)),
            gross_total: 0,
            net_total: 0,
            par_played: 0,
            score_to_par: 0,
        });
    }
    let points = |e: &RoundLeaderboardEntry| {
        e.stableford.as_ref().map_or(0, |v| match metric {
            LeaderboardMetric::Gross => v.gross_points,
            LeaderboardMetric::Net => v.net_points,
        })
    };
    entries.sort_by(|a, b| {
        (b.holes_scored > 0)
            .cmp(&(a.holes_scored > 0))
            .then_with(|| points(b).cmp(&points(a)))
            .then(b.holes_scored.cmp(&a.holes_scored))
            .then_with(|| name_cmp(&a.owner_name, &b.owner_name))
            .then(a.owner.id().cmp(&b.owner.id()))
    });
    let mut offset = 0;
    for group in entries
        .chunk_by_mut(|a, b| (a.holes_scored > 0, points(a)) == (b.holes_scored > 0, points(b)))
    {
        let tied = group.len() > 1;
        for entry in group.iter_mut().filter(|e| e.holes_scored > 0) {
            entry.position = Some(offset + 1);
            entry.tied = tied;
        }
        offset += group.len();
    }
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
