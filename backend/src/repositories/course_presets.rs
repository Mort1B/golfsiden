use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::tournament_authorization::{self, AuthorizationError};

#[derive(Clone, Copy, Serialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "tee_category", rename_all = "lowercase")]
pub enum PresetCategory {
    Male,
    Female,
}

#[derive(Serialize)]
pub struct CoursePreset {
    pub id: Uuid,
    pub course_name: String,
    pub location: Option<String>,
    pub tee: PresetTee,
}
#[derive(Serialize)]
pub struct PresetTee {
    pub category: PresetCategory,
    pub name: String,
    pub course_rating: f64,
    pub slope_rating: i16,
    pub holes: Vec<PresetHole>,
}
#[derive(Serialize)]
pub struct PresetHole {
    pub number: i16,
    pub par: i16,
    pub stroke_index: i16,
    pub distance: Option<i16>,
}
#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    course_name: String,
    location: Option<String>,
    category: PresetCategory,
    tee_name: String,
    course_rating: f64,
    slope_rating: i16,
    number: i16,
    par: i16,
    stroke_index: i16,
    distance: Option<i16>,
}

pub async fn list_for_admin(
    pool: &PgPool,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<Vec<CoursePreset>, AuthorizationError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    tournament_authorization::require_tournament_member_read(&mut tx, user_id, tournament_id)
        .await?;
    tournament_authorization::require_tournament_admin_read_in_transaction(
        &mut tx,
        user_id,
        tournament_id,
    )
    .await?;
    let rows = sqlx::query_as::<_, Row>(
        "SELECT c.id, c.name AS course_name, c.location, t.category, t.name AS tee_name,
         t.course_rating::float8 AS course_rating, t.slope_rating, h.hole_number AS number,
         h.par, h.stroke_index, h.yardage AS distance
         FROM course_presets p JOIN courses c ON c.id=p.course_id
         JOIN tees t ON t.course_id=c.id JOIN holes h ON h.tee_id=t.id
         WHERE c.source='manual' ORDER BY p.display_order, c.id, h.hole_number",
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut presets: Vec<CoursePreset> = Vec::new();
    for row in rows {
        if presets.last().is_none_or(|preset| preset.id != row.id) {
            presets.push(CoursePreset {
                id: row.id,
                course_name: row.course_name,
                location: row.location,
                tee: PresetTee {
                    category: row.category,
                    name: row.tee_name,
                    course_rating: row.course_rating,
                    slope_rating: row.slope_rating,
                    holes: Vec::new(),
                },
            });
        }
        if let Some(preset) = presets.last_mut() {
            preset.tee.holes.push(PresetHole {
                number: row.number,
                par: row.par,
                stroke_index: row.stroke_index,
                distance: row.distance,
            });
        }
    }
    tx.commit().await?;
    Ok(presets)
}
