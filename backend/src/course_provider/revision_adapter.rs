use thiserror::Error;

use crate::domain::course_revisions::{
    self, CourseRevisionCommand, CourseRevisionSource, CourseRevisionValidationError,
    HoleRevisionCommand, TeeCategory as RevisionTeeCategory, TeeRevisionCommand,
    ValidatedCourseRevision,
};

use super::{CourseDetail, Tee, TeeCategory};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProviderRevisionError {
    #[error("the selected provider tee is no longer available")]
    TeeStale,
    #[error(transparent)]
    InvalidFacts(#[from] CourseRevisionValidationError),
}

pub fn select_and_validate(
    course: CourseDetail,
    category: TeeCategory,
    tee_name: &str,
) -> Result<ValidatedCourseRevision, ProviderRevisionError> {
    let selector = tee_name.trim();
    let CourseDetail {
        provider_course_id,
        course_name,
        location,
        tees,
        ..
    } = course;
    let mut matches = tees
        .into_iter()
        .filter(|tee| tee.category == category && tee.name.trim() == selector);
    let selected = matches.next().ok_or(ProviderRevisionError::TeeStale)?;
    if matches.next().is_some() {
        return Err(ProviderRevisionError::TeeStale);
    }

    let command = command(provider_course_id, course_name, location, selected)?;
    course_revisions::validate(command).map_err(ProviderRevisionError::InvalidFacts)
}

fn command(
    provider_course_id: String,
    course_name: String,
    course_location: super::CourseLocation,
    tee: Tee,
) -> Result<CourseRevisionCommand, ProviderRevisionError> {
    let location = [
        course_location.address.as_deref(),
        course_location.city.as_deref(),
        course_location.state.as_deref(),
        course_location.country.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join(", ");
    let slope_rating = i16::try_from(tee.slope_rating).map_err(|_| {
        ProviderRevisionError::InvalidFacts(CourseRevisionValidationError::InvalidSlopeRating)
    })?;
    let holes = tee
        .holes
        .into_iter()
        .map(|hole| {
            Ok(HoleRevisionCommand {
                par: i16::try_from(hole.par).map_err(|_| {
                    ProviderRevisionError::InvalidFacts(CourseRevisionValidationError::InvalidPar)
                })?,
                stroke_index: i16::try_from(hole.stroke_index).map_err(|_| {
                    ProviderRevisionError::InvalidFacts(
                        CourseRevisionValidationError::InvalidStrokeIndexes,
                    )
                })?,
                distance: Some(i16::try_from(hole.yardage).map_err(|_| {
                    ProviderRevisionError::InvalidFacts(
                        CourseRevisionValidationError::InvalidDistance,
                    )
                })?),
            })
        })
        .collect::<Result<Vec<_>, ProviderRevisionError>>()?;
    Ok(CourseRevisionCommand {
        source: CourseRevisionSource::GolfCourseApi,
        provider_course_id: Some(provider_course_id),
        course_name,
        location: (!location.is_empty()).then_some(location),
        tee: TeeRevisionCommand {
            category: match tee.category {
                TeeCategory::Female => RevisionTeeCategory::Female,
                TeeCategory::Male => RevisionTeeCategory::Male,
            },
            name: tee.name,
            course_rating: tee.course_rating,
            slope_rating,
            holes,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course_provider::{CourseLocation, Hole};

    fn course(tees: Vec<Tee>) -> CourseDetail {
        CourseDetail {
            provider: "golf_course_api",
            provider_course_id: "opaque-id".to_owned(),
            club_name: "Club".to_owned(),
            course_name: "Course".to_owned(),
            scorecard_url: None,
            location: CourseLocation {
                city: Some(" Oslo ".to_owned()),
                country: Some("Norway".to_owned()),
                ..CourseLocation::default()
            },
            tees,
        }
    }

    fn tee(name: &str) -> Tee {
        Tee {
            category: TeeCategory::Male,
            name: name.to_owned(),
            course_rating: 71.2,
            slope_rating: 125,
            total_yards: 700,
            total_meters: 640,
            number_of_holes: 2,
            par_total: 7,
            holes: vec![
                Hole {
                    number: 1,
                    par: 4,
                    yardage: 400,
                    stroke_index: 2,
                },
                Hole {
                    number: 2,
                    par: 3,
                    yardage: 300,
                    stroke_index: 1,
                },
            ],
        }
    }

    #[test]
    fn selects_one_trimmed_exact_tee_and_builds_server_owned_revision() {
        let validated =
            select_and_validate(course(vec![tee(" White ")]), TeeCategory::Male, "White")
                .expect("synthetic provider facts should validate");
        assert_eq!(validated.provider_course_id.as_deref(), Some("opaque-id"));
        assert_eq!(validated.course_name, "Course");
        assert_eq!(validated.location.as_deref(), Some("Oslo, Norway"));
        assert_eq!(validated.tee.holes[0].distance, Some(400));
    }

    #[test]
    fn rejects_missing_or_ambiguous_exact_selector() {
        assert_eq!(
            select_and_validate(course(vec![tee("Blue")]), TeeCategory::Male, "White").unwrap_err(),
            ProviderRevisionError::TeeStale
        );
        assert_eq!(
            select_and_validate(
                course(vec![tee(" White"), tee("White ")]),
                TeeCategory::Male,
                " White ",
            )
            .unwrap_err(),
            ProviderRevisionError::TeeStale
        );
    }

    #[test]
    fn fails_closed_instead_of_dropping_out_of_range_distance() {
        let mut invalid = tee("White");
        invalid.holes[0].yardage = i32::from(i16::MAX) + 1;
        assert!(matches!(
            select_and_validate(course(vec![invalid]), TeeCategory::Male, "White"),
            Err(ProviderRevisionError::InvalidFacts(
                CourseRevisionValidationError::InvalidDistance
            ))
        ));
    }
}
