use super::FourBallError;
use crate::domain::handicap::CalculatedHandicap;

pub const DEFAULT_ALLOWANCE_PERCENT: i16 = 85;
const COURSE_HANDICAP_DENOMINATOR: i128 = 1_130;

/// Accepts the uncapped, unrounded numerator produced by the existing tee formula.
/// This new policy is deliberately separate from legacy format calculations.
pub fn calculate_handicap(
    course_handicap_numerator: i64,
    allowance_percent: i16,
    handicap_enabled: bool,
) -> Result<CalculatedHandicap, FourBallError> {
    if !(0..=100).contains(&allowance_percent) {
        return Err(FourBallError::InvalidAllowance);
    }
    if !handicap_enabled {
        return Ok(CalculatedHandicap {
            course_handicap: 0,
            playing_handicap: 0,
        });
    }
    // Widen before multiplication: every i64 numerator and valid allowance fits.
    let numerator = i128::from(course_handicap_numerator);
    Ok(CalculatedHandicap {
        course_handicap: rounded_snapshot(numerator, COURSE_HANDICAP_DENOMINATOR)?,
        playing_handicap: rounded_snapshot(
            numerator * i128::from(allowance_percent),
            COURSE_HANDICAP_DENOMINATOR * 100,
        )?,
    })
}

fn rounded_snapshot(numerator: i128, denominator: i128) -> Result<i16, FourBallError> {
    let rounded = numerator.div_euclid(denominator)
        + i128::from(numerator.rem_euclid(denominator) * 2 >= denominator);
    i16::try_from(rounded).map_err(|_| FourBallError::HandicapOutOfRange)
}
