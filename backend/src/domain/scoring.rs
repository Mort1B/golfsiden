use uuid::Uuid;

use super::models::RoundStatus;

pub trait TeamHandicapFormula {
    fn playing_handicap(&self, course_handicaps: &[i32]) -> Result<i32, ScoringError>;
}

#[derive(Debug, Default)]
pub struct TwoPlayerScramble35And15;

impl TeamHandicapFormula for TwoPlayerScramble35And15 {
    fn playing_handicap(&self, course_handicaps: &[i32]) -> Result<i32, ScoringError> {
        let [first, second] = course_handicaps else {
            return Err(ScoringError::InvalidTeamSize);
        };
        let (lower, higher) = if first <= second {
            (*first, *second)
        } else {
            (*second, *first)
        };
        Ok(
            round_ratio_half_away_from_zero(i64::from(lower) * 35 + i64::from(higher) * 15, 100)
                as i32,
        )
    }
}

pub fn scramble_playing_handicap(
    course_handicaps: &[i32],
    allowance_percent: i16,
) -> Result<i32, ScoringError> {
    let team_handicap = TwoPlayerScramble35And15.playing_handicap(course_handicaps)?;
    Ok(
        round_ratio_half_away_from_zero(
            i64::from(team_handicap) * i64::from(allowance_percent),
            100,
        ) as i32,
    )
}

const COURSE_HANDICAP_DENOMINATOR: i64 = 1_130;
const FOURSOMES_ALLOWANCE_PERCENT: i64 = 50;

pub fn foursomes_playing_handicap(course_handicap_numerators: &[i64]) -> Result<i32, ScoringError> {
    let [first, second] = course_handicap_numerators else {
        return Err(ScoringError::InvalidTeamSize);
    };
    let numerator = first
        .checked_add(*second)
        .and_then(|sum| sum.checked_mul(FOURSOMES_ALLOWANCE_PERCENT))
        .ok_or(ScoringError::ArithmeticOverflow)?;
    let rounded =
        round_ratio_half_toward_positive_infinity(numerator, COURSE_HANDICAP_DENOMINATOR * 100);
    i32::try_from(rounded).map_err(|_| ScoringError::ArithmeticOverflow)
}

fn round_ratio_half_toward_positive_infinity(numerator: i64, denominator: i64) -> i64 {
    let quotient = numerator.div_euclid(denominator);
    let remainder = numerator.rem_euclid(denominator);
    quotient + i64::from(remainder * 2 >= denominator)
}

fn round_ratio_half_away_from_zero(numerator: i64, denominator: i64) -> i64 {
    let sign = numerator.signum();
    let absolute = numerator.abs();
    let quotient = absolute / denominator;
    let remainder = absolute % denominator;
    sign * (quotient + i64::from(remainder * 2 >= denominator))
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ScoringError {
    #[error("the initial scramble formula requires exactly two players")]
    InvalidTeamSize,
    #[error("handicap arithmetic exceeded its supported range")]
    ArithmeticOverflow,
    #[error("a score must belong to either one player or one team")]
    InvalidScoreOwner,
    #[error("ordinary score changes are not allowed after a round is locked")]
    RoundLocked,
    #[error("hole number and stroke index must be within the configured hole count")]
    InvalidHole,
}

pub fn gross_total(hole_scores: &[i32]) -> i32 {
    hole_scores.iter().sum()
}

pub fn net_total(gross: i32, playing_handicap: i32) -> i32 {
    gross - playing_handicap
}

pub fn handicap_strokes_for_hole(
    playing_handicap: i32,
    stroke_index: i32,
    number_of_holes: i32,
) -> Result<i32, ScoringError> {
    if number_of_holes <= 0 || !(1..=number_of_holes).contains(&stroke_index) {
        return Err(ScoringError::InvalidHole);
    }
    if playing_handicap >= 0 {
        let base = playing_handicap / number_of_holes;
        let remainder = playing_handicap % number_of_holes;
        Ok(base + i32::from(stroke_index <= remainder))
    } else {
        let absolute = playing_handicap.unsigned_abs() as i32;
        let base = absolute / number_of_holes;
        let remainder = absolute % number_of_holes;
        Ok(-(base + i32::from(stroke_index > number_of_holes - remainder)))
    }
}

pub fn hole_net_score(
    gross: i32,
    playing_handicap: i32,
    stroke_index: i32,
    number_of_holes: i32,
) -> Result<i32, ScoringError> {
    Ok(gross - handicap_strokes_for_hole(playing_handicap, stroke_index, number_of_holes)?)
}

pub fn validate_score_owner(
    player_id: Option<Uuid>,
    team_id: Option<Uuid>,
) -> Result<(), ScoringError> {
    match (player_id, team_id) {
        (Some(_), None) | (None, Some(_)) => Ok(()),
        _ => Err(ScoringError::InvalidScoreOwner),
    }
}

pub fn require_score_editable(
    status: RoundStatus,
    admin_correction: bool,
) -> Result<(), ScoringError> {
    if status == RoundStatus::Locked && !admin_correction {
        return Err(ScoringError::RoundLocked);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_two_player_scramble_handicap() {
        assert_eq!(TwoPlayerScramble35And15.playing_handicap(&[8, 20]), Ok(6));
        assert_eq!(TwoPlayerScramble35And15.playing_handicap(&[20, 8]), Ok(6));
        assert_eq!(
            TwoPlayerScramble35And15.playing_handicap(&[8]),
            Err(ScoringError::InvalidTeamSize)
        );
        assert_eq!(
            TwoPlayerScramble35And15.playing_handicap(&[-10, -10]),
            Ok(-5)
        );
        assert_eq!(scramble_playing_handicap(&[8, 20], 75), Ok(5));
        assert_eq!(scramble_playing_handicap(&[5, 5], 50), Ok(2));
    }

    #[test]
    fn foursomes_applies_fifty_percent_once_without_intermediate_rounding() {
        assert_eq!(foursomes_playing_handicap(&[16_385, 2_825]), Ok(9));
        // Rounding the members first would yield (1 + 2) * 50% = 1.5 -> 2.
        assert_eq!(foursomes_playing_handicap(&[678, 1_808]), Ok(1));
    }

    #[test]
    fn foursomes_half_rounding_moves_plus_handicaps_toward_zero() {
        assert_eq!(foursomes_playing_handicap(&[2_260, 3_390]), Ok(3));
        assert_eq!(foursomes_playing_handicap(&[-2_260, -3_390]), Ok(-2));
    }

    #[test]
    fn foursomes_extreme_inputs_return_overflow_instead_of_panicking() {
        assert_eq!(
            foursomes_playing_handicap(&[i64::MAX, 1]),
            Err(ScoringError::ArithmeticOverflow)
        );
        assert_eq!(
            foursomes_playing_handicap(&[i64::MAX / 2, 0]),
            Err(ScoringError::ArithmeticOverflow)
        );
    }

    #[test]
    fn calculates_gross_and_total_deduction_net_scores() {
        assert_eq!(gross_total(&[4, 5, 3, 4]), 16);
        assert_eq!(net_total(88, 14), 74);
    }

    #[test]
    fn allocates_handicap_strokes_by_stroke_index() {
        assert_eq!(handicap_strokes_for_hole(20, 1, 18), Ok(2));
        assert_eq!(handicap_strokes_for_hole(20, 2, 18), Ok(2));
        assert_eq!(handicap_strokes_for_hole(20, 3, 18), Ok(1));
        assert_eq!(hole_net_score(6, 20, 1, 18), Ok(4));
        assert_eq!(handicap_strokes_for_hole(-2, 18, 18), Ok(-1));
    }

    #[test]
    fn score_owner_is_exclusive() {
        let player = Uuid::new_v4();
        let team = Uuid::new_v4();
        assert!(validate_score_owner(Some(player), None).is_ok());
        assert_eq!(
            validate_score_owner(Some(player), Some(team)),
            Err(ScoringError::InvalidScoreOwner)
        );
        assert_eq!(
            validate_score_owner(None, None),
            Err(ScoringError::InvalidScoreOwner)
        );
    }

    #[test]
    fn locked_round_requires_admin_correction() {
        assert_eq!(
            require_score_editable(RoundStatus::Locked, false),
            Err(ScoringError::RoundLocked)
        );
        assert!(require_score_editable(RoundStatus::Locked, true).is_ok());
    }

    #[test]
    fn historical_net_uses_the_preserved_round_handicap() {
        let preserved_round_handicap = 12;
        let current_handicap_after_change = 8;
        assert_eq!(net_total(84, preserved_round_handicap), 72);
        assert_ne!(
            net_total(84, preserved_round_handicap),
            net_total(84, current_handicap_after_change)
        );
    }
}
