use std::collections::HashSet;

use thiserror::Error;

const MAX_COURSE_NAME_BYTES: usize = 300;
const MAX_LOCATION_BYTES: usize = 500;
const MAX_TEE_NAME_BYTES: usize = 100;
const MAX_PROVIDER_ID_BYTES: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CourseRevisionSource {
    GolfCourseApi,
    Manual,
}

impl CourseRevisionSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::GolfCourseApi => "golf_course_api",
            Self::Manual => "manual",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeeCategory {
    Female,
    Male,
}

impl TeeCategory {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Female => "female",
            Self::Male => "male",
        }
    }
}

#[derive(Debug)]
pub struct CourseRevisionCommand {
    pub source: CourseRevisionSource,
    pub provider_course_id: Option<String>,
    pub course_name: String,
    pub location: Option<String>,
    pub tee: TeeRevisionCommand,
}

#[derive(Debug)]
pub struct TeeRevisionCommand {
    pub category: TeeCategory,
    pub name: String,
    pub course_rating: f64,
    pub slope_rating: i16,
    pub holes: Vec<HoleRevisionCommand>,
}

#[derive(Debug)]
pub struct HoleRevisionCommand {
    pub par: i16,
    pub stroke_index: i16,
    pub distance: Option<i16>,
}

#[derive(Debug)]
pub struct ValidatedCourseRevision {
    pub(crate) source: CourseRevisionSource,
    pub(crate) provider_course_id: Option<String>,
    pub(crate) course_name: String,
    pub(crate) location: Option<String>,
    pub(crate) tee: ValidatedTeeRevision,
}

#[derive(Debug)]
pub(crate) struct ValidatedTeeRevision {
    pub(crate) category: TeeCategory,
    pub(crate) name: String,
    pub(crate) course_rating_tenths: i16,
    pub(crate) slope_rating: i16,
    pub(crate) holes: Vec<HoleRevisionCommand>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CourseRevisionValidationError {
    #[error("course revision text is invalid")]
    InvalidText,
    #[error("provider provenance is invalid")]
    InvalidProvenance,
    #[error("course rating must be from 1.0 through 100.0 with at most one decimal place")]
    InvalidCourseRating,
    #[error("slope rating must be between 55 and 155")]
    InvalidSlopeRating,
    #[error("a course revision must contain between 1 and 36 holes")]
    InvalidHoleCount,
    #[error("hole par must be between 2 and 7")]
    InvalidPar,
    #[error("stroke indexes must be a complete unique permutation")]
    InvalidStrokeIndexes,
    #[error("hole distance must be positive when provided")]
    InvalidDistance,
}

pub fn validate(
    command: CourseRevisionCommand,
) -> Result<ValidatedCourseRevision, CourseRevisionValidationError> {
    let course_name = validated_text(&command.course_name, MAX_COURSE_NAME_BYTES)?;
    let location = command
        .location
        .map(|value| validated_text(&value, MAX_LOCATION_BYTES))
        .transpose()?;
    let provider_course_id = match (command.source, command.provider_course_id) {
        (CourseRevisionSource::GolfCourseApi, Some(value)) => {
            Some(validated_text(&value, MAX_PROVIDER_ID_BYTES)?)
        }
        (CourseRevisionSource::Manual, None) => None,
        _ => return Err(CourseRevisionValidationError::InvalidProvenance),
    };
    let tee_name = validated_text(&command.tee.name, MAX_TEE_NAME_BYTES)?;
    let rating_tenths = rating_tenths(command.tee.course_rating)?;
    if !(55..=155).contains(&command.tee.slope_rating) {
        return Err(CourseRevisionValidationError::InvalidSlopeRating);
    }
    if !(1..=36).contains(&command.tee.holes.len()) {
        return Err(CourseRevisionValidationError::InvalidHoleCount);
    }

    let hole_count = i16::try_from(command.tee.holes.len())
        .map_err(|_| CourseRevisionValidationError::InvalidHoleCount)?;
    let mut stroke_indexes = HashSet::with_capacity(command.tee.holes.len());
    for hole in &command.tee.holes {
        if !(2..=7).contains(&hole.par) {
            return Err(CourseRevisionValidationError::InvalidPar);
        }
        if !(1..=hole_count).contains(&hole.stroke_index)
            || !stroke_indexes.insert(hole.stroke_index)
        {
            return Err(CourseRevisionValidationError::InvalidStrokeIndexes);
        }
        if matches!(hole.distance, Some(distance) if distance <= 0) {
            return Err(CourseRevisionValidationError::InvalidDistance);
        }
    }

    Ok(ValidatedCourseRevision {
        source: command.source,
        provider_course_id,
        course_name,
        location,
        tee: ValidatedTeeRevision {
            category: command.tee.category,
            name: tee_name,
            course_rating_tenths: rating_tenths,
            slope_rating: command.tee.slope_rating,
            holes: command.tee.holes,
        },
    })
}

fn validated_text(value: &str, max_bytes: usize) -> Result<String, CourseRevisionValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_bytes || trimmed.chars().any(char::is_control) {
        return Err(CourseRevisionValidationError::InvalidText);
    }
    Ok(trimmed.to_owned())
}

fn rating_tenths(value: f64) -> Result<i16, CourseRevisionValidationError> {
    if !value.is_finite() || !(1.0..=100.0).contains(&value) {
        return Err(CourseRevisionValidationError::InvalidCourseRating);
    }
    let tenths = value * 10.0;
    if (tenths - tenths.round()).abs() > 1e-9 {
        return Err(CourseRevisionValidationError::InvalidCourseRating);
    }
    Ok(tenths.round() as i16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(source: CourseRevisionSource) -> CourseRevisionCommand {
        CourseRevisionCommand {
            source,
            provider_course_id: (source == CourseRevisionSource::GolfCourseApi)
                .then(|| " opaque-ID_7 ".to_owned()),
            course_name: " Course ".to_owned(),
            location: Some(" Norway ".to_owned()),
            tee: TeeRevisionCommand {
                category: TeeCategory::Male,
                name: " White ".to_owned(),
                course_rating: 72.4,
                slope_rating: 128,
                holes: vec![
                    HoleRevisionCommand {
                        par: 4,
                        stroke_index: 2,
                        distance: Some(410),
                    },
                    HoleRevisionCommand {
                        par: 3,
                        stroke_index: 1,
                        distance: None,
                    },
                ],
            },
        }
    }

    #[test]
    fn validates_and_normalizes_both_provenance_paths() {
        let provider = validate(command(CourseRevisionSource::GolfCourseApi)).unwrap();
        assert_eq!(provider.provider_course_id.as_deref(), Some("opaque-ID_7"));
        assert_eq!(provider.course_name, "Course");
        assert_eq!(provider.tee.course_rating_tenths, 724);

        let manual = validate(command(CourseRevisionSource::Manual)).unwrap();
        assert_eq!(manual.provider_course_id, None);
    }

    #[test]
    fn rejects_invalid_provenance_and_incomplete_stroke_indexes() {
        let mut manual = command(CourseRevisionSource::Manual);
        manual.provider_course_id = Some("invented".to_owned());
        assert_eq!(
            validate(manual).unwrap_err(),
            CourseRevisionValidationError::InvalidProvenance
        );

        let mut duplicate = command(CourseRevisionSource::GolfCourseApi);
        duplicate.tee.holes[0].stroke_index = 1;
        assert_eq!(
            validate(duplicate).unwrap_err(),
            CourseRevisionValidationError::InvalidStrokeIndexes
        );
    }

    #[test]
    fn rejects_invalid_ranges_and_text() {
        let mut invalid = command(CourseRevisionSource::Manual);
        invalid.course_name = "\n".to_owned();
        assert_eq!(
            validate(invalid).unwrap_err(),
            CourseRevisionValidationError::InvalidText
        );

        let mut invalid = command(CourseRevisionSource::Manual);
        invalid.tee.course_rating = 72.45;
        assert_eq!(
            validate(invalid).unwrap_err(),
            CourseRevisionValidationError::InvalidCourseRating
        );

        let mut invalid = command(CourseRevisionSource::Manual);
        invalid.tee.holes[0].distance = Some(0);
        assert_eq!(
            validate(invalid).unwrap_err(),
            CourseRevisionValidationError::InvalidDistance
        );
    }
}
