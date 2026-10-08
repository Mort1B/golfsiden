use super::{GolferRound, MatchOutcome, OwnerFacts, Settlement};
use crate::domain::{
    fantasy::{
        FantasyError, GolferId, Points,
        matches::{MatchSource, match_points},
    },
    match_play::{
        self, ConfirmationStatus, HandicapAllocation, MatchMode, Opponent,
        commands::{AcceptedEvent, validate_ledger},
    },
};
pub fn project_match(input: &OwnerFacts, hidden: bool) -> Result<GolferRound, FantasyError> {
    let f = &input.facts;
    let golfer = GolferId(f.owner_id);
    if hidden {
        return Ok(GolferRound {
            golfer,
            points: Points::Withheld,
            team_id: None,
            holes: vec![],
            placement_points: None,
            settlement: Settlement::Withheld,
            match_outcome: None,
        });
    }
    let (state, confirmation, opponent) = if let Some(m) = f.matches.first() {
        if f.matches.len() != 1 {
            return Err(FantasyError::InvalidFacts);
        }
        let playing = |p| {
            f.snapshots
                .iter()
                .find(|s| s.player_id == p)
                .map(|s| s.playing_handicap)
                .ok_or(FantasyError::InvalidFacts)
        };
        let allocation = HandicapAllocation::new(
            if f.round.handicap_enabled {
                MatchMode::Net
            } else {
                MatchMode::Gross
            },
            [playing(m.first_player_id)?, playing(m.second_player_id)?],
        );
        let events: Vec<AcceptedEvent> =
            serde_json::from_value(m.ledger.clone()).map_err(|_| FantasyError::InvalidFacts)?;
        let indexes = f
            .holes
            .iter()
            .map(|h| u8::try_from(h.stroke_index).map_err(|_| FantasyError::InvalidFacts))
            .collect::<Result<Vec<_>, _>>()?;
        let state = validate_ledger(
            &events,
            [m.first_player_id, m.second_player_id],
            allocation,
            &indexes,
            true,
        )
        .map_err(|_| FantasyError::InvalidFacts)?;
        let opponent = if f.owner_id == m.first_player_id {
            Opponent::First
        } else if f.owner_id == m.second_player_id {
            Opponent::Second
        } else {
            return Err(FantasyError::InvalidFacts);
        };
        (
            state,
            if m.confirmed {
                ConfirmationStatus::Confirmed
            } else {
                ConfirmationStatus::Unconfirmed
            },
            opponent,
        )
    } else {
        (
            match_play::derive_match(&[]).map_err(|_| FantasyError::InvalidFacts)?,
            ConfirmationStatus::Unconfirmed,
            Opponent::First,
        )
    };
    let points = match_points(
        MatchSource::Visible {
            state,
            confirmation,
            source: input.source,
            non_finish: input.non_finish,
        },
        opponent,
    );
    let outcome = if state.finish().is_some() {
        points.total().and_then(|p| match p {
            3 => Some(MatchOutcome::Win),
            1 => Some(MatchOutcome::Draw),
            -1 => Some(MatchOutcome::Loss),
            _ => None,
        })
    } else {
        None
    };
    let settlement = if state.finish().is_some() {
        if confirmation == ConfirmationStatus::Confirmed {
            Settlement::Confirmed
        } else {
            Settlement::Unconfirmed
        }
    } else if input.non_finish == Some(input.source) {
        Settlement::NonFinish
    } else if input.non_finish.is_some() {
        Settlement::StaleNonFinish
    } else {
        Settlement::Playing
    };
    Ok(GolferRound {
        golfer,
        points,
        team_id: None,
        holes: vec![],
        placement_points: None,
        settlement,
        match_outcome: outcome,
    })
}
