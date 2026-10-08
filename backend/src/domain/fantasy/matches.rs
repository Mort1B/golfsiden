use super::{Points, SourceToken};
use crate::domain::match_play::{ConfirmationStatus, MatchFinish, MatchState, Opponent};

/// Accepted sporting state only. Numeric notes are deliberately not an input.
#[derive(Debug, Clone, Copy)]
pub enum MatchSource {
    Visible {
        state: MatchState,
        confirmation: ConfirmationStatus,
        source: SourceToken,
        non_finish: Option<SourceToken>,
    },
    Withheld,
}

pub fn match_points(source: MatchSource, golfer: Opponent) -> Points {
    let MatchSource::Visible {
        state,
        confirmation,
        source,
        non_finish,
    } = source
    else {
        return Points::Withheld;
    };
    let Some(finish) = state.finish() else {
        return if non_finish == Some(source) {
            Points::Settled(0)
        } else {
            Points::Pending { recorded: 0 }
        };
    };
    let points = match finish {
        MatchFinish::Draw => 1,
        MatchFinish::OnHoles { winner, .. }
        | MatchFinish::Conceded { winner }
        | MatchFinish::Awarded { winner } => {
            if winner == golfer {
                3
            } else {
                -1
            }
        }
    };
    match confirmation {
        ConfirmationStatus::Confirmed => Points::Settled(points),
        ConfirmationStatus::Unconfirmed => Points::Provisional(points),
    }
}
