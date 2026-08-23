use std::collections::{HashMap, HashSet};

use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::{CourseProviderError, normalize_course_id};

const PROVIDER: &str = "golf_course_api";

#[derive(Clone, Debug, Serialize)]
pub struct CourseSearchResult {
    pub provider: &'static str,
    pub provider_course_id: String,
    pub club_name: String,
    pub course_name: String,
    pub scorecard_url: Option<String>,
    pub location: CourseLocation,
    pub tee_counts: TeeCounts,
}

#[derive(Clone, Debug, Serialize)]
pub struct CourseDetail {
    pub provider: &'static str,
    pub provider_course_id: String,
    pub club_name: String,
    pub course_name: String,
    pub scorecard_url: Option<String>,
    pub location: CourseLocation,
    pub tees: Vec<Tee>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CourseLocation {
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TeeCounts {
    pub female: u32,
    pub male: u32,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TeeCategory {
    Female,
    Male,
}

#[derive(Clone, Debug, Serialize)]
pub struct Tee {
    pub category: TeeCategory,
    pub name: String,
    pub course_rating: f64,
    pub slope_rating: i32,
    pub total_yards: i32,
    pub total_meters: i32,
    pub number_of_holes: i32,
    pub par_total: i32,
    pub holes: Vec<Hole>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Hole {
    pub number: usize,
    pub par: i32,
    pub yardage: i32,
    pub stroke_index: i32,
}

#[derive(Deserialize)]
pub(super) struct SearchEnvelope {
    pub courses: Vec<ProviderCourseSummary>,
}

#[derive(Deserialize)]
pub(super) struct ProviderCourseSummary {
    id: String,
    club_name: String,
    course_name: String,
    scorecard_url: Option<String>,
    #[serde(default)]
    location: CourseLocation,
    #[serde(default)]
    tees: HashMap<String, u32>,
}

impl ProviderCourseSummary {
    pub(super) fn normalize(self) -> Result<CourseSearchResult, CourseProviderError> {
        validate_text(&self.club_name, 300)?;
        validate_text(&self.course_name, 300)?;
        validate_url(self.scorecard_url.as_deref())?;
        self.location.validate()?;
        let female = self.tees.get("female").copied().unwrap_or_default();
        let male = self.tees.get("male").copied().unwrap_or_default();
        if female > 100 || male > 100 {
            return Err(CourseProviderError::InvalidResponse);
        }
        Ok(CourseSearchResult {
            provider: PROVIDER,
            provider_course_id: normalize_course_id(&self.id)
                .map_err(|_| CourseProviderError::InvalidResponse)?,
            club_name: self.club_name,
            course_name: self.course_name,
            scorecard_url: self.scorecard_url,
            location: self.location,
            tee_counts: TeeCounts { female, male },
        })
    }
}

#[derive(Deserialize)]
pub(super) struct ProviderCourse {
    id: String,
    club_name: String,
    course_name: String,
    scorecard_url: Option<String>,
    #[serde(default)]
    location: CourseLocation,
    #[serde(default)]
    tees: ProviderTees,
}

#[derive(Default, Deserialize)]
struct ProviderTees {
    #[serde(default)]
    female: Vec<ProviderTee>,
    #[serde(default)]
    male: Vec<ProviderTee>,
}

#[derive(Deserialize)]
struct ProviderTee {
    tee_name: String,
    course_rating: f64,
    slope_rating: i32,
    total_yards: i32,
    total_meters: i32,
    number_of_holes: i32,
    par_total: i32,
    #[serde(default)]
    holes: Vec<ProviderHole>,
}

#[derive(Deserialize)]
struct ProviderHole {
    par: i32,
    yardage: i32,
    handicap: i32,
}

impl ProviderCourse {
    pub(super) fn normalize(self, requested_id: &str) -> Result<CourseDetail, CourseProviderError> {
        let provider_course_id =
            normalize_course_id(&self.id).map_err(|_| CourseProviderError::InvalidResponse)?;
        if provider_course_id != requested_id {
            return Err(CourseProviderError::InvalidResponse);
        }
        validate_text(&self.club_name, 300)?;
        validate_text(&self.course_name, 300)?;
        validate_url(self.scorecard_url.as_deref())?;
        self.location.validate()?;
        if self.tees.female.len() + self.tees.male.len() > 100 {
            return Err(CourseProviderError::InvalidResponse);
        }
        let mut tees = Vec::with_capacity(self.tees.female.len() + self.tees.male.len());
        for tee in self.tees.female {
            tees.push(tee.normalize(TeeCategory::Female)?);
        }
        for tee in self.tees.male {
            tees.push(tee.normalize(TeeCategory::Male)?);
        }
        Ok(CourseDetail {
            provider: PROVIDER,
            provider_course_id,
            club_name: self.club_name,
            course_name: self.course_name,
            scorecard_url: self.scorecard_url,
            location: self.location,
            tees,
        })
    }
}

impl ProviderTee {
    fn normalize(self, category: TeeCategory) -> Result<Tee, CourseProviderError> {
        validate_text(&self.tee_name, 100)?;
        if !self.course_rating.is_finite()
            || !(1.0..=100.0).contains(&self.course_rating)
            || !(1..=200).contains(&self.slope_rating)
            || self.total_yards < 0
            || self.total_meters < 0
            || !(1..=36).contains(&self.number_of_holes)
            || self.holes.len() != self.number_of_holes as usize
        {
            return Err(CourseProviderError::InvalidResponse);
        }
        let mut stroke_indexes = HashSet::with_capacity(self.holes.len());
        let mut par_total = 0;
        let mut holes = Vec::with_capacity(self.holes.len());
        for (index, hole) in self.holes.into_iter().enumerate() {
            if !(2..=7).contains(&hole.par)
                || !(1..=2_000).contains(&hole.yardage)
                || !(1..=36).contains(&hole.handicap)
                || !stroke_indexes.insert(hole.handicap)
            {
                return Err(CourseProviderError::InvalidResponse);
            }
            par_total += hole.par;
            holes.push(Hole {
                number: index + 1,
                par: hole.par,
                yardage: hole.yardage,
                stroke_index: hole.handicap,
            });
        }
        if par_total != self.par_total {
            return Err(CourseProviderError::InvalidResponse);
        }
        Ok(Tee {
            category,
            name: self.tee_name,
            course_rating: self.course_rating,
            slope_rating: self.slope_rating,
            total_yards: self.total_yards,
            total_meters: self.total_meters,
            number_of_holes: self.number_of_holes,
            par_total: self.par_total,
            holes,
        })
    }
}

impl CourseLocation {
    fn validate(&self) -> Result<(), CourseProviderError> {
        for value in [&self.address, &self.city, &self.state, &self.country]
            .into_iter()
            .flatten()
        {
            if value.len() > 500 || value.chars().any(char::is_control) {
                return Err(CourseProviderError::InvalidResponse);
            }
        }
        Ok(())
    }
}

fn validate_text(value: &str, max_bytes: usize) -> Result<(), CourseProviderError> {
    if value.trim().is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(CourseProviderError::InvalidResponse);
    }
    Ok(())
}

fn validate_url(value: Option<&str>) -> Result<(), CourseProviderError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.len() > 2_048 {
        return Err(CourseProviderError::InvalidResponse);
    }
    let parsed = Url::parse(value).map_err(|_| CourseProviderError::InvalidResponse)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(CourseProviderError::InvalidResponse);
    }
    Ok(())
}
