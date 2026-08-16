use std::collections::HashMap;

use sqlx::{FromRow, PgConnection, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        handicap::{calculate, effective_index_tenths},
        models::{
            OpenRoundResult, PairingValidation, ParticipantStatus, Round, RoundHandicapSnapshot,
            RoundStatus, ScoringFormat, TournamentStatus,
        },
        round_lifecycle::{ConfigurationFact, EntrantFact, ReadinessFacts, TeamFact, validate},
    },
    repositories::tournament_authorization::{self, AuthorizationError},
};

const ROUND_COLUMNS: &str = "id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format, created_at, updated_at";

#[derive(Debug, Error)]
pub enum OpenRoundError {
    #[error("round not found")]
    NotFound,
    #[error("round is not ready to open")]
    NotReady(PairingValidation),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

#[derive(FromRow)]
struct RoundFactRow {
    id: Uuid,
    tournament_id: Uuid,
    status: RoundStatus,
    scoring_format: ScoringFormat,
    handicap_enabled: bool,
    handicap_allowance_percent: i16,
    number_of_holes: i16,
    tee_id: Option<Uuid>,
    course_id: Option<Uuid>,
}

#[derive(FromRow)]
struct TeeFactRow {
    course_id: Uuid,
    slope_rating: Option<i16>,
    course_rating_tenths: Option<i32>,
}

#[derive(FromRow)]
struct HoleFactRow {
    hole_number: i16,
    stroke_index: i16,
    par: i16,
}

#[derive(FromRow)]
struct EntrantRow {
    player_id: Uuid,
    display_name: String,
    participant_status: ParticipantStatus,
    player_active: bool,
    handicap_index_tenths: i32,
}

#[derive(FromRow)]
struct TeamRow {
    team_id: Uuid,
    team_name: String,
}

#[derive(FromRow)]
struct MembershipRow {
    team_id: Uuid,
    player_id: Uuid,
}

struct LoadedRound {
    facts: ReadinessFacts,
    tournament_id: Uuid,
    handicap_allowance_percent: i16,
    course_par: i16,
}

pub async fn pairing_validation(
    pool: &PgPool,
    round_id: Uuid,
) -> Result<Option<PairingValidation>, sqlx::Error> {
    let mut connection = pool.acquire().await?;
    Ok(load(&mut connection, round_id, false)
        .await?
        .map(|loaded| validate(&loaded.facts)))
}

pub async fn open(pool: &PgPool, round_id: Uuid) -> Result<OpenRoundResult, OpenRoundError> {
    let mut transaction = pool.begin().await?;
    let result = open_in_transaction(&mut transaction, round_id).await?;
    transaction.commit().await?;
    Ok(result)
}

pub async fn open_authorized(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
) -> Result<OpenRoundResult, OpenRoundError> {
    let mut transaction = pool.begin().await?;
    tournament_authorization::require_round_admin(&mut transaction, session_id, round_id).await?;
    let result = open_in_transaction(&mut transaction, round_id).await?;
    transaction.commit().await?;
    Ok(result)
}

async fn open_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
) -> Result<OpenRoundResult, OpenRoundError> {
    let loaded = load(transaction, round_id, true)
        .await?
        .ok_or(OpenRoundError::NotFound)?;
    let validation = validate(&loaded.facts);
    if !validation.ready {
        return Err(OpenRoundError::NotReady(validation));
    }

    sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
        .bind(round_id)
        .execute(&mut **transaction)
        .await?;

    let slope_rating = loaded.facts.configuration.slope_rating.unwrap_or(113);
    let course_rating_tenths = loaded
        .facts
        .configuration
        .course_rating_tenths
        .unwrap_or(i32::from(loaded.course_par) * 10);
    for entrant in loaded.facts.entrants.iter().filter(|entrant| {
        entrant.participant_status == ParticipantStatus::Active && entrant.player_active
    }) {
        let effective_index_tenths =
            effective_index_tenths(loaded.facts.scoring_format, entrant.handicap_index_tenths);
        let handicap = calculate(
            effective_index_tenths,
            slope_rating,
            course_rating_tenths,
            loaded.course_par,
            loaded.handicap_allowance_percent,
            loaded.facts.handicap_enabled,
            loaded.facts.scoring_format,
        );
        sqlx::query("INSERT INTO round_handicap_snapshots (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap) VALUES ($1, $2, $3, $4::numeric / 10, $5, $6)")
            .bind(round_id)
            .bind(loaded.tournament_id)
            .bind(entrant.player_id)
            .bind(effective_index_tenths)
            .bind(handicap.course_handicap)
            .bind(handicap.playing_handicap)
            .execute(&mut **transaction)
            .await?;
    }

    sqlx::query("UPDATE rounds SET status = 'open' WHERE id = $1 AND status = 'draft'")
        .bind(round_id)
        .execute(&mut **transaction)
        .await?;
    let round =
        sqlx::query_as::<_, Round>(&format!("SELECT {ROUND_COLUMNS} FROM rounds WHERE id = $1"))
            .bind(round_id)
            .fetch_one(&mut **transaction)
            .await?;
    let handicap_snapshots = sqlx::query_as::<_, RoundHandicapSnapshot>(
        "SELECT round_id, tournament_id, player_id, handicap_index::float8 AS handicap_index, course_handicap, playing_handicap, captured_at FROM round_handicap_snapshots WHERE round_id = $1 ORDER BY player_id",
    )
    .bind(round_id)
    .fetch_all(&mut **transaction)
    .await?;
    Ok(OpenRoundResult {
        round,
        handicap_snapshots,
    })
}

async fn load(
    connection: &mut PgConnection,
    round_id: Uuid,
    lock_for_open: bool,
) -> Result<Option<LoadedRound>, sqlx::Error> {
    let round_sql = if lock_for_open {
        "SELECT id, tournament_id, status, scoring_format, handicap_enabled, handicap_allowance_percent, number_of_holes, tee_id, course_id FROM rounds WHERE id = $1 FOR UPDATE"
    } else {
        "SELECT id, tournament_id, status, scoring_format, handicap_enabled, handicap_allowance_percent, number_of_holes, tee_id, course_id FROM rounds WHERE id = $1"
    };
    let Some(round) = sqlx::query_as::<_, RoundFactRow>(round_sql)
        .bind(round_id)
        .fetch_optional(&mut *connection)
        .await?
    else {
        return Ok(None);
    };

    let tournament_sql = if lock_for_open {
        // The stronger lock also serializes new tournament-player foreign-key checks.
        "SELECT status FROM tournaments WHERE id = $1 FOR UPDATE"
    } else {
        "SELECT status FROM tournaments WHERE id = $1"
    };
    let tournament_status = sqlx::query_scalar::<_, TournamentStatus>(tournament_sql)
        .bind(round.tournament_id)
        .fetch_one(&mut *connection)
        .await?;
    let tee = load_tee(connection, round.tee_id).await?;
    let holes = load_holes(connection, round.tee_id).await?;
    let entrants = load_entrants(connection, round.tournament_id, lock_for_open).await?;
    let teams = load_teams(connection, round.id).await?;
    let course_par = holes.iter().map(|hole| hole.par).sum();
    let configuration = ConfigurationFact {
        course_id: round.course_id,
        tee_id: round.tee_id,
        tee_course_id: tee.as_ref().map(|tee| tee.course_id),
        slope_rating: tee.as_ref().and_then(|tee| tee.slope_rating),
        course_rating_tenths: tee.as_ref().and_then(|tee| tee.course_rating_tenths),
        hole_numbers: holes.iter().map(|hole| hole.hole_number).collect(),
        stroke_indexes: holes.iter().map(|hole| hole.stroke_index).collect(),
    };

    Ok(Some(LoadedRound {
        facts: ReadinessFacts {
            round_id: round.id,
            round_status: round.status,
            tournament_status,
            scoring_format: round.scoring_format,
            handicap_enabled: round.handicap_enabled,
            number_of_holes: round.number_of_holes,
            entrants,
            teams,
            configuration,
        },
        tournament_id: round.tournament_id,
        handicap_allowance_percent: round.handicap_allowance_percent,
        course_par,
    }))
}

async fn load_tee(
    connection: &mut PgConnection,
    tee_id: Option<Uuid>,
) -> Result<Option<TeeFactRow>, sqlx::Error> {
    let Some(tee_id) = tee_id else {
        return Ok(None);
    };
    sqlx::query_as(
        "SELECT course_id, slope_rating, (course_rating * 10)::int4 AS course_rating_tenths FROM tees WHERE id = $1",
    )
        .bind(tee_id)
        .fetch_optional(&mut *connection)
        .await
}

async fn load_holes(
    connection: &mut PgConnection,
    tee_id: Option<Uuid>,
) -> Result<Vec<HoleFactRow>, sqlx::Error> {
    let Some(tee_id) = tee_id else {
        return Ok(Vec::new());
    };
    sqlx::query_as(
        "SELECT hole_number, stroke_index, par FROM holes WHERE tee_id = $1 ORDER BY hole_number",
    )
    .bind(tee_id)
    .fetch_all(&mut *connection)
    .await
}

async fn load_entrants(
    connection: &mut PgConnection,
    tournament_id: Uuid,
    lock_for_open: bool,
) -> Result<Vec<EntrantFact>, sqlx::Error> {
    let sql = if lock_for_open {
        "SELECT tp.player_id, p.display_name, tp.status AS participant_status, p.active AS player_active, (tp.tournament_handicap * 10)::int4 AS handicap_index_tenths FROM tournament_players tp JOIN players p ON p.id = tp.player_id WHERE tp.tournament_id = $1 ORDER BY p.display_name, tp.player_id FOR SHARE OF tp, p"
    } else {
        "SELECT tp.player_id, p.display_name, tp.status AS participant_status, p.active AS player_active, (tp.tournament_handicap * 10)::int4 AS handicap_index_tenths FROM tournament_players tp JOIN players p ON p.id = tp.player_id WHERE tp.tournament_id = $1 ORDER BY p.display_name, tp.player_id"
    };
    Ok(sqlx::query_as::<_, EntrantRow>(sql)
        .bind(tournament_id)
        .fetch_all(&mut *connection)
        .await?
        .into_iter()
        .map(|row| EntrantFact {
            player_id: row.player_id,
            display_name: row.display_name,
            participant_status: row.participant_status,
            player_active: row.player_active,
            handicap_index_tenths: row.handicap_index_tenths,
        })
        .collect())
}

async fn load_teams(
    connection: &mut PgConnection,
    round_id: Uuid,
) -> Result<Vec<TeamFact>, sqlx::Error> {
    let team_rows = sqlx::query_as::<_, TeamRow>(
        "SELECT id AS team_id, name AS team_name FROM teams WHERE round_id = $1 ORDER BY name, id",
    )
    .bind(round_id)
    .fetch_all(&mut *connection)
    .await?;
    let memberships = sqlx::query_as::<_, MembershipRow>(
        "SELECT team_id, player_id FROM team_memberships WHERE round_id = $1 ORDER BY team_id, player_id",
    )
    .bind(round_id)
    .fetch_all(&mut *connection)
    .await?;
    let mut members_by_team: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for membership in memberships {
        members_by_team
            .entry(membership.team_id)
            .or_default()
            .push(membership.player_id);
    }
    Ok(team_rows
        .into_iter()
        .map(|team| TeamFact {
            team_id: team.team_id,
            team_name: team.team_name,
            player_ids: members_by_team.remove(&team.team_id).unwrap_or_default(),
        })
        .collect())
}
