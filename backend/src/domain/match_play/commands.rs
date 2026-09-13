//! Typed agreement provenance. Numeric notes never become accepted reports implicitly.
use super::{HandicapAllocation, HoleOutcome, MatchReport, MatchState, Opponent, derive_match};
use crate::domain::{player_score_input::GrossScore, scorecards::ScoreRevision};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    First,
    Second,
    Halved,
}
impl Outcome {
    pub fn domain(self) -> HoleOutcome {
        match self {
            Self::First => HoleOutcome::WonBy(Opponent::First),
            Self::Second => HoleOutcome::WonBy(Opponent::Second),
            Self::Halved => HoleOutcome::Halved,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Basis {
    Numeric {
        first_gross: i16,
        second_gross: i16,
        agreed: bool,
    },
    NextStrokeConcession {
        first_gross: i16,
        second_gross: i16,
        conceding_player_id: Uuid,
        communicated: bool,
        agreed: bool,
    },
    HoleConcession {
        conceding_player_id: Uuid,
        communicated: bool,
    },
    AgreedHalve {
        play_begun: bool,
        mutual_agreement: bool,
    },
    OrganizerRuling {
        reason: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Hole {
        hole_number: u8,
        outcome: Outcome,
        basis: Basis,
    },
    Concession {
        conceding_player_id: Uuid,
        communicated: bool,
        after_hole: u8,
    },
    Award {
        winner_player_id: Uuid,
        reason: String,
        after_hole: u8,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedEvent {
    pub id: Uuid,
    pub event: Event,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionKind {
    RecordingError,
    OrganizerRuling,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Note {
        player_id: Uuid,
        hole_number: u8,
        gross_strokes: i16,
    },
    ClearNote {
        player_id: Uuid,
        hole_number: u8,
    },
    Report {
        event: Event,
    },
    Confirm {
        result_agreed_or_awarded: bool,
    },
    Correct {
        kind: CorrectionKind,
        reason: String,
        superseded_event_ids: Vec<Uuid>,
        replacement: Vec<Event>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub request_id: Uuid,
    pub expected_revision: ScoreRevision,
    pub command: Command,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acknowledgement {
    pub request_id: Uuid,
    pub match_id: Uuid,
    pub applied_revision: ScoreRevision,
}

pub fn reason_valid(reason: &str) -> bool {
    !reason.trim().is_empty() && reason.len() <= 1000 && !reason.contains('\0')
}
fn opponent(id: Uuid, players: [Uuid; 2]) -> Result<Opponent, &'static str> {
    let [first, second] = players;
    if id == first {
        Ok(Opponent::First)
    } else if id == second {
        Ok(Opponent::Second)
    } else {
        Err("player is not an opponent")
    }
}
pub fn validate_ledger(
    events: &[AcceptedEvent],
    players: [Uuid; 2],
    allocation: HandicapAllocation,
    stroke_indexes: &[u8],
    admin: bool,
) -> Result<MatchState, &'static str> {
    let mut reports = Vec::new();
    for accepted in events {
        let state = derive_match(&reports).map_err(|_| "invalid accepted sequence")?;
        let report = match &accepted.event {
            Event::Hole {
                hole_number,
                outcome,
                basis,
            } => {
                let si = hole_number
                    .checked_sub(1)
                    .and_then(|i| stroke_indexes.get(usize::from(i)))
                    .copied()
                    .ok_or("invalid hole")?;
                match basis {
                    Basis::Numeric {
                        first_gross,
                        second_gross,
                        agreed,
                    }
                    | Basis::NextStrokeConcession {
                        first_gross,
                        second_gross,
                        agreed,
                        ..
                    } => {
                        if !agreed {
                            return Err("numeric outcome requires agreement");
                        }
                        let gross =
                            [*first_gross, *second_gross].map(|n| GrossScore::new(i32::from(n)));
                        let [a, b] = gross;
                        let proposed = allocation
                            .propose_numeric_hole(
                                [
                                    a.map_err(|_| "invalid numeric score")?,
                                    b.map_err(|_| "invalid numeric score")?,
                                ],
                                si,
                            )
                            .map_err(|_| "invalid hole allocation")?;
                        if proposed != outcome.domain() {
                            return Err("numeric evidence does not support outcome");
                        }
                        if let Basis::NextStrokeConcession {
                            conceding_player_id,
                            communicated,
                            ..
                        } = basis
                        {
                            opponent(*conceding_player_id, players)?;
                            if !communicated {
                                return Err("concession requires communication attestation");
                            }
                        }
                    }
                    Basis::HoleConcession {
                        conceding_player_id,
                        communicated,
                    } => {
                        let conceder = opponent(*conceding_player_id, players)?;
                        if !communicated || outcome.domain() != HoleOutcome::WonBy(conceder.other())
                        {
                            return Err("invalid communicated hole concession");
                        }
                    }
                    Basis::AgreedHalve {
                        play_begun,
                        mutual_agreement,
                    } => {
                        if !play_begun || !mutual_agreement || *outcome != Outcome::Halved {
                            return Err("halve requires play begun and mutual agreement");
                        }
                    }
                    Basis::OrganizerRuling { reason } => {
                        if !admin || !reason_valid(reason) {
                            return Err("organizer ruling requires admin and reason");
                        }
                    }
                }
                MatchReport::Hole {
                    number: *hole_number,
                    outcome: outcome.domain(),
                }
            }
            Event::Concession {
                conceding_player_id,
                communicated,
                after_hole,
            } => {
                if !communicated || *after_hole != state.resolved_holes() {
                    return Err("invalid concession effective point");
                }
                MatchReport::Conceded {
                    by: opponent(*conceding_player_id, players)?,
                }
            }
            Event::Award {
                winner_player_id,
                reason,
                after_hole,
            } => {
                if !admin || !reason_valid(reason) || *after_hole != state.resolved_holes() {
                    return Err("award requires admin, reason and current effective point");
                }
                MatchReport::Awarded {
                    winner: opponent(*winner_player_id, players)?,
                }
            }
        };
        reports.push(report);
    }
    derive_match(&reports).map_err(
        |_| "reports must form an unfinished contiguous prefix followed by at most one finish",
    )
}
pub fn visible_event(event: &Event, hole_limit: u8) -> bool {
    match event {
        Event::Hole { hole_number, .. } => *hole_number <= hole_limit,
        Event::Concession { after_hole, .. } | Event::Award { after_hole, .. } => {
            *after_hole < hole_limit
        }
    }
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
