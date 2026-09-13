use super::*;
use crate::domain::{
    models::{ParticipantStatus, RoundStatus, ScoringFormat, TournamentTieBreakPolicy},
    player_score_input::{GrossScore, PlayerHoleInput},
    score_visibility::{VisibilityMetadata, VisibilityMode, unrestricted},
};
use uuid::Uuid;
fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}
fn round(
    n: i16,
    format: ScoringFormat,
    handicaps: [i16; 2],
    bogeys: [usize; 2],
) -> RoundLeaderboardFacts {
    let rid = id(100 + n as u128);
    let mut f = RoundLeaderboardFacts {
        stableford_inputs: vec![],
        round: RoundFact {
            round_id: rid,
            tournament_id: id(99),
            round_number: n,
            status: RoundStatus::Completed,
            scoring_format: format,
            number_of_holes: 18,
            handicap_enabled: true,
            handicap_allowance_percent: 100,
        },
        holes: vec![],
        snapshots: vec![],
        team_snapshots: vec![],
        teams: vec![],
        memberships: vec![],
        scores: vec![],
        confirmations: vec![],
    };
    for hole in 1..=18 {
        f.holes.push(HoleFact {
            round_id: rid,
            hole_id: id(1000 + n as u128 * 100 + hole),
            hole_number: hole as i16,
            par: 4,
            stroke_index: hole as i16,
        });
    }
    for (index, hcp) in handicaps.into_iter().enumerate() {
        let player = id(index as u128 + 1);
        f.snapshots.push(SnapshotFact {
            round_id: rid,
            player_id: player,
            display_name: format!("Player {index}"),
            course_handicap: hcp,
            playing_handicap: hcp,
        });
        for (h, hole) in f.holes.iter().enumerate() {
            let gross = if h < bogeys[index] { 5 } else { 4 };
            if format == ScoringFormat::IndividualStableford {
                f.stableford_inputs.push(StablefordInputFact {
                    round_id: rid,
                    hole_id: hole.hole_id,
                    player_id: player,
                    input: PlayerHoleInput::Numeric(GrossScore::new(gross).unwrap()),
                });
            } else {
                f.scores.push(ScoreFact {
                    round_id: rid,
                    hole_id: hole.hole_id,
                    player_id: Some(player),
                    team_id: None,
                    gross_strokes: gross as i16,
                });
            }
        }
        if format == ScoringFormat::FourBallStrokePlay {
            f.memberships.push(MembershipFact {
                round_id: rid,
                team_id: id(50),
                player_id: player,
                display_name: format!("Player {index}"),
                display_order: None,
            });
        }
    }
    if format == ScoringFormat::FourBallStrokePlay {
        f.teams.push(TeamFact {
            round_id: rid,
            team_id: id(50),
            team_name: "Side".into(),
        });
        for score in &mut f.scores {
            if score.hole_id == id(1000 + n as u128 * 100 + 1) {
                score.gross_strokes = 3;
            }
        }
    }
    f
}
fn tournament(rounds: Vec<RoundLeaderboardFacts>) -> TournamentLeaderboardFacts {
    TournamentLeaderboardFacts {
        final_round_number: rounds.len() as i16,
        tie_break_policy: TournamentTieBreakPolicy::SharedPositions,
        tournament_id: id(99),
        counted_rounds: 2,
        mandatory_round_id: None,
        participants: (1..=2)
            .map(|p| ParticipantFact {
                player_id: id(p),
                display_name: format!("Player {}", p - 1),
                status: ParticipantStatus::Active,
            })
            .collect(),
        rounds,
    }
}
#[test]
fn mixed_best_n_uses_equivalents_independently_and_preserves_native_units() {
    let mut facts = tournament(vec![
        round(1, ScoringFormat::IndividualStrokePlay, [3, 3], [4, 3]),
        round(2, ScoringFormat::IndividualStableford, [8, 4], [4, 2]),
        round(3, ScoringFormat::FourBallStrokePlay, [2, 2], [0, 0]),
    ]);
    for (metric, expected) in [
        (LeaderboardMetric::Gross, [(1, 3), (2, 1)]),
        (LeaderboardMetric::Net, [(1, -7), (2, -5)]),
    ] {
        let board = build_tournament_leaderboard(&facts, metric).unwrap();
        for (player, total) in expected {
            let entry = board
                .entries
                .iter()
                .find(|e| e.player_id == id(player))
                .unwrap();
            assert_eq!(entry.score_to_par, total);
            assert!(entry.equivalents.is_some());
            assert_eq!(entry.contributions.len(), 3);
        }
        let json = serde_json::to_value(&board).unwrap();
        assert!(json["entries"][0].get("gross_total").is_none());
        assert_eq!(json["entries"][0]["value"]["type"], "overall_equivalent");
    }
    let native = build_round_leaderboard(&facts.rounds[1], LeaderboardMetric::Net).unwrap();
    assert_eq!(
        native.entries[0].owner,
        LeaderboardOwner::Player { id: id(1) }
    );
    assert_eq!(
        native.entries[0].stableford.as_ref().unwrap().net_points,
        40
    );
    let json = serde_json::to_value(native).unwrap();
    assert!(json["entries"][0].get("gross_total").is_none());
    assert_eq!(json["entries"][0]["value"]["actual_gross_total"], 76);
    facts.counted_rounds = 1;
    facts.mandatory_round_id = Some(id(102));
    let board = build_tournament_leaderboard(&facts, LeaderboardMetric::Net).unwrap();
    assert_eq!(board.entries[0].score_to_par, -4);
    assert!(
        board
            .entries
            .iter()
            .all(|e| e.contributions.iter().find(|c| c.counted).unwrap().round_id == id(102))
    );
}
#[test]
fn zero_points_rank_but_blank_cards_have_no_contribution_and_hidden_projection_is_invariant() {
    let mut sf = round(1, ScoringFormat::IndividualStableford, [0, 0], [0, 0]);
    sf.round.status = RoundStatus::Open;
    sf.stableford_inputs.retain(|i| i.player_id == id(1));
    for i in &mut sf.stableford_inputs {
        i.input = PlayerHoleInput::NoScore;
    }
    let board = build_round_leaderboard(&sf, LeaderboardMetric::Gross).unwrap();
    assert_eq!(board.entries[0].position, Some(1));
    assert_eq!(
        board.entries[0]
            .stableford
            .as_ref()
            .unwrap()
            .gross_equivalent,
        36
    );
    assert_eq!(board.entries[1].position, None);
    let mut facts = tournament(vec![sf.clone()]);
    facts.counted_rounds = 1;
    let overall = build_tournament_leaderboard(&facts, LeaderboardMetric::Gross).unwrap();
    assert_eq!(overall.entries[0].score_to_par, 36);
    assert!(overall.entries[1].contributions.is_empty());
    let visibility = VisibilityMetadata {
        mode: VisibilityMode::FrontNine,
    };
    let before = serde_json::to_value(
        build_round_leaderboard_projected(&sf, LeaderboardMetric::Gross, visibility).unwrap(),
    )
    .unwrap();
    for i in sf
        .stableford_inputs
        .iter_mut()
        .filter(|i| i.hole_id > id(1109))
    {
        i.input = PlayerHoleInput::Numeric(GrossScore::new(1).unwrap());
    }
    let after = serde_json::to_value(
        build_round_leaderboard_projected(&sf, LeaderboardMetric::Gross, visibility).unwrap(),
    )
    .unwrap();
    assert_eq!(before, after);
}
#[test]
fn final_stableford_outside_best_n_breaks_equal_overall_by_lower_equivalent() {
    let first = round(1, ScoringFormat::IndividualStrokePlay, [6, 6], [0, 0]);
    let last = round(2, ScoringFormat::IndividualStableford, [4, 2], [0, 0]);
    let mut facts = tournament(vec![first, last]);
    facts.counted_rounds = 1;
    facts.tie_break_policy = TournamentTieBreakPolicy::FinalRoundScore;
    let board = build_tournament_leaderboard(&facts, LeaderboardMetric::Net).unwrap();
    assert_eq!(board.entries[0].score_to_par, -6);
    assert_eq!(board.entries[1].score_to_par, -6);
    assert_eq!(board.entries[0].tie_break_score_to_par, Some(-4));
    assert_eq!(board.entries[1].tie_break_score_to_par, Some(-2));
    assert_eq!(board.entries[0].position, Some(1));
    assert_eq!(board.entries[1].position, Some(2));
    let board = build_tournament_leaderboard_projected(
        &facts,
        LeaderboardMetric::Net,
        unrestricted(),
        Some(id(102)),
        unrestricted(),
    )
    .unwrap();
    assert!(
        board
            .entries
            .iter()
            .all(|e| e.tie_break_score_to_par.is_none() && e.position == Some(1))
    );
}
#[test]
fn nine_visible_holes_twenty_points_gives_minus_two_and_legacy_serialization_is_unchanged() {
    let mut sf = round(1, ScoringFormat::IndividualStableford, [0, 0], [0, 0]);
    sf.round.status = RoundStatus::Open;
    for input in sf
        .stableford_inputs
        .iter_mut()
        .filter(|i| i.hole_id == id(1101))
    {
        input.input = PlayerHoleInput::Numeric(GrossScore::new(2).unwrap());
    }
    let board = build_round_leaderboard_projected(
        &sf,
        LeaderboardMetric::Gross,
        VisibilityMetadata {
            mode: VisibilityMode::FrontNine,
        },
    )
    .unwrap();
    for entry in board.entries {
        let v = entry.stableford.unwrap();
        assert_eq!(
            (
                entry.holes_scored,
                v.gross_points,
                v.gross_equivalent,
                v.actual_gross_total
            ),
            (9, 20, -2, None)
        );
    }
    let stroke = build_round_leaderboard(
        &round(1, ScoringFormat::IndividualStrokePlay, [0, 0], [0, 0]),
        LeaderboardMetric::Gross,
    )
    .unwrap();
    let json = serde_json::to_value(stroke).unwrap();
    assert!(json["entries"][0].get("value").is_none());
    assert_eq!(json["entries"][0]["gross_total"], 72);
    assert_eq!(json["entries"][0]["score_to_par"], 0);
}
