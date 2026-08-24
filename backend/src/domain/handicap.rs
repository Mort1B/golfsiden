use super::{
    models::ScoringFormat,
    round_formats::{RoundFormatPolicy, SnapshotHandicapPolicy},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalculatedHandicap {
    pub course_handicap: i16,
    pub playing_handicap: i16,
}

pub fn effective_index_tenths(scoring_format: ScoringFormat, registered_tenths: i32) -> i32 {
    RoundFormatPolicy::for_format(scoring_format).effective_index_tenths(registered_tenths)
}

pub fn calculate(
    handicap_index_tenths: i32,
    slope_rating: i16,
    course_rating_tenths: i32,
    course_par: i16,
    allowance_percent: i16,
    handicap_enabled: bool,
    scoring_format: ScoringFormat,
) -> CalculatedHandicap {
    if !handicap_enabled {
        return CalculatedHandicap {
            course_handicap: 0,
            playing_handicap: 0,
        };
    }

    let numerator = course_handicap_numerator(
        handicap_index_tenths,
        slope_rating,
        course_rating_tenths,
        course_par,
    );
    let denominator = 1_130;
    let course_handicap = round_ratio_half_away_from_zero(numerator, denominator);
    let playing_handicap = match RoundFormatPolicy::for_format(scoring_format).snapshot_handicap() {
        SnapshotHandicapPolicy::UncappedIndividualRoundAllowance => {
            round_ratio_half_away_from_zero(
                numerator * i64::from(allowance_percent),
                denominator * 100,
            )
        }
        SnapshotHandicapPolicy::UncappedCourseHandicap => course_handicap,
        SnapshotHandicapPolicy::IndexCappedCourseHandicap { .. } => course_handicap,
    };

    CalculatedHandicap {
        course_handicap: course_handicap as i16,
        playing_handicap: playing_handicap as i16,
    }
}

pub fn course_handicap_numerator(
    handicap_index_tenths: i32,
    slope_rating: i16,
    course_rating_tenths: i32,
    course_par: i16,
) -> i64 {
    i64::from(handicap_index_tenths) * i64::from(slope_rating)
        + i64::from(course_rating_tenths - i32::from(course_par) * 10) * 113
}

fn round_ratio_half_away_from_zero(numerator: i64, denominator: i64) -> i64 {
    let sign = numerator.signum();
    let absolute = numerator.abs();
    let quotient = absolute / denominator;
    let remainder = absolute % denominator;
    sign * (quotient + i64::from(remainder * 2 >= denominator))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scramble_caps_before_non_neutral_tee_conversion() {
        let results = [359, 360, 361, 540].map(|registered| {
            let effective = effective_index_tenths(ScoringFormat::TeamScramble, registered);
            (
                effective,
                calculate(
                    effective,
                    155,
                    722,
                    72,
                    95,
                    true,
                    ScoringFormat::TeamScramble,
                ),
            )
        });
        assert_eq!(results.map(|result| result.0), [359, 360, 360, 360]);
        assert_eq!(
            results.map(|result| result.1.course_handicap),
            [49, 50, 50, 50]
        );
    }

    #[test]
    fn individual_retains_full_registered_index() {
        assert_eq!(
            effective_index_tenths(ScoringFormat::IndividualStrokePlay, 540),
            540
        );
        let handicap = calculate(
            540,
            155,
            722,
            72,
            95,
            true,
            ScoringFormat::IndividualStrokePlay,
        );
        assert_eq!(handicap.course_handicap, 74);
        assert_eq!(handicap.playing_handicap, 71);
    }

    #[test]
    fn rounding_and_allowance_use_the_unrounded_value() {
        assert_eq!(
            calculate(
                0,
                113,
                725,
                72,
                100,
                true,
                ScoringFormat::IndividualStrokePlay
            )
            .course_handicap,
            1
        );
        assert_eq!(
            calculate(
                0,
                113,
                715,
                72,
                100,
                true,
                ScoringFormat::IndividualStrokePlay
            )
            .course_handicap,
            -1
        );
        assert_eq!(
            calculate(
                96,
                113,
                720,
                72,
                95,
                true,
                ScoringFormat::IndividualStrokePlay
            )
            .playing_handicap,
            9
        );
    }
}
