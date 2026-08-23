use std::collections::HashSet;

use super::{ReadinessFacts, push_if};
use crate::domain::models::{ReadinessIssue, ReadinessIssueCode};

pub(super) fn validate(facts: &ReadinessFacts, issues: &mut Vec<ReadinessIssue>) {
    let configuration = &facts.configuration;
    push_if(
        issues,
        configuration.course_id.is_none(),
        ReadinessIssueCode::MissingCourse,
        "round requires a course",
    );
    push_if(
        issues,
        configuration.tee_id.is_none(),
        ReadinessIssueCode::MissingTee,
        "round requires a tee",
    );
    push_if(
        issues,
        configuration.course_id.is_some()
            && configuration.tee_id.is_some()
            && configuration.tee_course_id != configuration.course_id,
        ReadinessIssueCode::MismatchedCourseTee,
        "tee must belong to the round course",
    );
    push_if(
        issues,
        facts.handicap_enabled
            && (configuration.slope_rating.is_none()
                || configuration.course_rating_tenths.is_none()),
        ReadinessIssueCode::MissingHandicapRatings,
        "handicap-enabled rounds require slope and course ratings",
    );
    push_if(
        issues,
        configuration.hole_numbers.len() != facts.number_of_holes as usize,
        ReadinessIssueCode::InvalidHoleCount,
        "tee must have exactly the configured number of holes",
    );
    push_if(
        issues,
        !is_complete_permutation(&configuration.hole_numbers, facts.number_of_holes),
        ReadinessIssueCode::InvalidHoleNumbers,
        "hole numbers must be the complete range for the round",
    );
    push_if(
        issues,
        !is_complete_permutation(&configuration.stroke_indexes, facts.number_of_holes),
        ReadinessIssueCode::InvalidStrokeIndexes,
        "stroke indexes must be the complete range for the round",
    );
}

fn is_complete_permutation(values: &[i16], expected_count: i16) -> bool {
    if expected_count < 1 || values.len() != expected_count as usize {
        return false;
    }
    let unique = values.iter().copied().collect::<HashSet<_>>();
    (1..=expected_count).all(|value| unique.contains(&value)) && unique.len() == values.len()
}
