use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::course_revisions::{CourseRevisionSource, TeeCategory, ValidatedCourseRevision};

#[derive(Debug, PartialEq)]
pub struct CourseRevision {
    pub course_id: Uuid,
    pub source: CourseRevisionSource,
    pub provider_course_id: Option<String>,
    pub course_name: String,
    pub location: Option<String>,
    pub imported_at: DateTime<Utc>,
    pub tee: TeeRevision,
}

#[derive(Debug, PartialEq)]
pub struct TeeRevision {
    pub tee_id: Uuid,
    pub category: TeeCategory,
    pub name: String,
    pub course_rating_tenths: i16,
    pub slope_rating: i16,
    pub holes: Vec<HoleRevision>,
}

#[derive(Debug, PartialEq)]
pub struct HoleRevision {
    pub hole_id: Uuid,
    pub number: i16,
    pub par: i16,
    pub stroke_index: i16,
    pub distance: Option<i16>,
}

#[derive(Debug, Error)]
pub enum CourseRevisionRepositoryError {
    #[error("course revision was not found")]
    NotFound,
    #[error("stored course revision is invalid")]
    InvalidStoredRevision,
    #[error("database operation failed")]
    Database(#[source] sqlx::Error),
}

pub async fn insert_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    revision: &ValidatedCourseRevision,
) -> Result<CourseRevision, CourseRevisionRepositoryError> {
    let course_id = Uuid::new_v4();
    let tee_id = Uuid::new_v4();
    sqlx::query("INSERT INTO courses (id, name, location) VALUES ($1, $2, $3)")
        .bind(course_id)
        .bind(&revision.course_name)
        .bind(&revision.location)
        .execute(&mut **transaction)
        .await
        .map_err(CourseRevisionRepositoryError::Database)?;

    let hole_count = i16::try_from(revision.tee.holes.len())
        .map_err(|_| CourseRevisionRepositoryError::InvalidStoredRevision)?;
    sqlx::query(
        "INSERT INTO tees
           (id, course_id, name, category, number_of_holes, slope_rating, course_rating)
         VALUES ($1, $2, $3, $4::tee_category, $5, $6, $7::numeric / 10)",
    )
    .bind(tee_id)
    .bind(course_id)
    .bind(&revision.tee.name)
    .bind(revision.tee.category.as_str())
    .bind(hole_count)
    .bind(revision.tee.slope_rating)
    .bind(revision.tee.course_rating_tenths)
    .execute(&mut **transaction)
    .await
    .map_err(CourseRevisionRepositoryError::Database)?;

    let mut holes = Vec::with_capacity(revision.tee.holes.len());
    for (index, hole) in revision.tee.holes.iter().enumerate() {
        let number = i16::try_from(index + 1)
            .map_err(|_| CourseRevisionRepositoryError::InvalidStoredRevision)?;
        let hole_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO holes (id, tee_id, hole_number, par, stroke_index, yardage)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(hole_id)
        .bind(tee_id)
        .bind(number)
        .bind(hole.par)
        .bind(hole.stroke_index)
        .bind(hole.distance)
        .execute(&mut **transaction)
        .await
        .map_err(CourseRevisionRepositoryError::Database)?;
        holes.push(HoleRevision {
            hole_id,
            number,
            par: hole.par,
            stroke_index: hole.stroke_index,
            distance: hole.distance,
        });
    }

    let imported_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "UPDATE courses
         SET source = $2::course_revision_source, provider_course_id = $3,
             imported_at = clock_timestamp()
         WHERE id = $1
         RETURNING imported_at",
    )
    .bind(course_id)
    .bind(revision.source.as_str())
    .bind(&revision.provider_course_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(CourseRevisionRepositoryError::Database)?;

    Ok(CourseRevision {
        course_id,
        source: revision.source,
        provider_course_id: revision.provider_course_id.clone(),
        course_name: revision.course_name.clone(),
        location: revision.location.clone(),
        imported_at,
        tee: TeeRevision {
            tee_id,
            category: revision.tee.category,
            name: revision.tee.name.clone(),
            course_rating_tenths: revision.tee.course_rating_tenths,
            slope_rating: revision.tee.slope_rating,
            holes,
        },
    })
}

pub async fn find_by_course_id(
    pool: &PgPool,
    course_id: Uuid,
) -> Result<CourseRevision, CourseRevisionRepositoryError> {
    let row = sqlx::query_as::<_, StoredRevisionRow>(
        "SELECT c.source::text AS source, c.provider_course_id, c.name AS course_name,
                c.location, c.imported_at, t.id AS tee_id, t.category::text AS category,
                t.name AS tee_name, (t.course_rating * 10)::int2 AS course_rating_tenths,
                t.slope_rating
         FROM courses c
         JOIN tees t ON t.course_id = c.id
         WHERE c.id = $1 AND c.source IS NOT NULL",
    )
    .bind(course_id)
    .fetch_optional(pool)
    .await
    .map_err(CourseRevisionRepositoryError::Database)?
    .ok_or(CourseRevisionRepositoryError::NotFound)?;
    let holes = sqlx::query_as::<_, StoredHoleRow>(
        "SELECT id, hole_number, par, stroke_index, yardage
         FROM holes WHERE tee_id = $1 ORDER BY hole_number",
    )
    .bind(row.tee_id)
    .fetch_all(pool)
    .await
    .map_err(CourseRevisionRepositoryError::Database)?;

    assemble_revision(course_id, row, holes)
}

#[derive(sqlx::FromRow)]
struct StoredRevisionRow {
    source: String,
    provider_course_id: Option<String>,
    course_name: String,
    location: Option<String>,
    imported_at: DateTime<Utc>,
    tee_id: Uuid,
    category: String,
    tee_name: String,
    course_rating_tenths: i16,
    slope_rating: i16,
}

#[derive(sqlx::FromRow)]
struct StoredHoleRow {
    id: Uuid,
    hole_number: i16,
    par: i16,
    stroke_index: i16,
    yardage: Option<i16>,
}

fn assemble_revision(
    course_id: Uuid,
    row: StoredRevisionRow,
    holes: Vec<StoredHoleRow>,
) -> Result<CourseRevision, CourseRevisionRepositoryError> {
    let source = match row.source.as_str() {
        "golf_course_api" => CourseRevisionSource::GolfCourseApi,
        "manual" => CourseRevisionSource::Manual,
        _ => return Err(CourseRevisionRepositoryError::InvalidStoredRevision),
    };
    let category = match row.category.as_str() {
        "female" => TeeCategory::Female,
        "male" => TeeCategory::Male,
        _ => return Err(CourseRevisionRepositoryError::InvalidStoredRevision),
    };
    let holes = holes
        .into_iter()
        .map(|hole| HoleRevision {
            hole_id: hole.id,
            number: hole.hole_number,
            par: hole.par,
            stroke_index: hole.stroke_index,
            distance: hole.yardage,
        })
        .collect();
    Ok(CourseRevision {
        course_id,
        source,
        provider_course_id: row.provider_course_id,
        course_name: row.course_name,
        location: row.location,
        imported_at: row.imported_at,
        tee: TeeRevision {
            tee_id: row.tee_id,
            category,
            name: row.tee_name,
            course_rating_tenths: row.course_rating_tenths,
            slope_rating: row.slope_rating,
            holes,
        },
    })
}
