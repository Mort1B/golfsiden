use std::collections::HashMap;

use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::domain::{
    models::{ParticipantStatus, RoundStatus, ScoringFormat, TournamentStatus},
    round_lifecycle::{ConfigurationFact, EntrantFact, FlightFact, ReadinessFacts, TeamFact},
};

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
struct GroupRow {
    group_id: Uuid,
    group_name: String,
}

#[derive(FromRow)]
struct MembershipRow {
    group_id: Uuid,
    player_id: Uuid,
}

pub(super) struct LoadedRound {
    pub facts: ReadinessFacts,
    pub tournament_id: Uuid,
    pub handicap_allowance_percent: i16,
    pub course_par: i16,
}

pub(super) async fn round(
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
        "SELECT status FROM tournaments WHERE id = $1 FOR UPDATE"
    } else {
        "SELECT status FROM tournaments WHERE id = $1"
    };
    let tournament_status = sqlx::query_scalar::<_, TournamentStatus>(tournament_sql)
        .bind(round.tournament_id)
        .fetch_one(&mut *connection)
        .await?;
    let entrants = load_entrants(connection, round.tournament_id, lock_for_open).await?;
    let teams = load_teams(connection, round.id).await?;
    let flights = load_flights(connection, round.id).await?;
    let tee = load_tee(connection, round.tee_id).await?;
    let holes = load_holes(connection, round.tee_id).await?;
    let course_par = holes.iter().map(|hole| hole.par).sum();
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
            flights,
            configuration: ConfigurationFact {
                course_id: round.course_id,
                tee_id: round.tee_id,
                tee_course_id: tee.as_ref().map(|tee| tee.course_id),
                slope_rating: tee.as_ref().and_then(|tee| tee.slope_rating),
                course_rating_tenths: tee.as_ref().and_then(|tee| tee.course_rating_tenths),
                hole_numbers: holes.iter().map(|hole| hole.hole_number).collect(),
                stroke_indexes: holes.iter().map(|hole| hole.stroke_index).collect(),
            },
        },
        tournament_id: round.tournament_id,
        handicap_allowance_percent: round.handicap_allowance_percent,
        course_par,
    }))
}

async fn load_entrants(
    connection: &mut PgConnection,
    tournament_id: Uuid,
    lock_for_open: bool,
) -> Result<Vec<EntrantFact>, sqlx::Error> {
    let sql = if lock_for_open {
        "SELECT tp.player_id, p.display_name, tp.status AS participant_status, p.active AS player_active, (tp.tournament_handicap * 10)::int4 AS handicap_index_tenths FROM tournament_players tp JOIN players p ON p.id = tp.player_id WHERE tp.tournament_id = $1 ORDER BY lower(p.display_name), tp.player_id FOR SHARE OF tp, p"
    } else {
        "SELECT tp.player_id, p.display_name, tp.status AS participant_status, p.active AS player_active, (tp.tournament_handicap * 10)::int4 AS handicap_index_tenths FROM tournament_players tp JOIN players p ON p.id = tp.player_id WHERE tp.tournament_id = $1 ORDER BY lower(p.display_name), tp.player_id"
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
    let (groups, memberships) = load_groups(
        connection,
        "SELECT id AS group_id, name AS group_name FROM teams WHERE round_id=$1 ORDER BY lower(name), id",
        "SELECT team_id AS group_id, player_id FROM team_memberships WHERE round_id=$1 ORDER BY team_id, display_order NULLS LAST, player_id",
        round_id,
    ).await?;
    Ok(groups
        .into_iter()
        .map(|group| TeamFact {
            team_id: group.group_id,
            team_name: group.group_name,
            player_ids: memberships
                .get(&group.group_id)
                .cloned()
                .unwrap_or_default(),
        })
        .collect())
}

async fn load_flights(
    connection: &mut PgConnection,
    round_id: Uuid,
) -> Result<Vec<FlightFact>, sqlx::Error> {
    let (groups, memberships) = load_groups(
        connection,
        "SELECT id AS group_id, name AS group_name FROM flights WHERE round_id=$1 ORDER BY lower(name), id",
        "SELECT flight_id AS group_id, player_id FROM flight_memberships WHERE round_id=$1 ORDER BY flight_id, display_order NULLS LAST, player_id",
        round_id,
    ).await?;
    Ok(groups
        .into_iter()
        .map(|group| FlightFact {
            flight_id: group.group_id,
            flight_name: group.group_name,
            player_ids: memberships
                .get(&group.group_id)
                .cloned()
                .unwrap_or_default(),
        })
        .collect())
}

async fn load_groups(
    connection: &mut PgConnection,
    groups_sql: &str,
    memberships_sql: &str,
    round_id: Uuid,
) -> Result<(Vec<GroupRow>, HashMap<Uuid, Vec<Uuid>>), sqlx::Error> {
    let groups = sqlx::query_as(groups_sql)
        .bind(round_id)
        .fetch_all(&mut *connection)
        .await?;
    let rows = sqlx::query_as::<_, MembershipRow>(memberships_sql)
        .bind(round_id)
        .fetch_all(&mut *connection)
        .await?;
    let mut memberships: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for row in rows {
        memberships
            .entry(row.group_id)
            .or_default()
            .push(row.player_id);
    }
    Ok((groups, memberships))
}

async fn load_tee(
    connection: &mut PgConnection,
    tee_id: Option<Uuid>,
) -> Result<Option<TeeFactRow>, sqlx::Error> {
    let Some(tee_id) = tee_id else {
        return Ok(None);
    };
    sqlx::query_as("SELECT course_id, slope_rating, (course_rating * 10)::int4 AS course_rating_tenths FROM tees WHERE id=$1")
        .bind(tee_id).fetch_optional(&mut *connection).await
}

async fn load_holes(
    connection: &mut PgConnection,
    tee_id: Option<Uuid>,
) -> Result<Vec<HoleFactRow>, sqlx::Error> {
    let Some(tee_id) = tee_id else {
        return Ok(Vec::new());
    };
    sqlx::query_as(
        "SELECT hole_number, stroke_index, par FROM holes WHERE tee_id=$1 ORDER BY hole_number",
    )
    .bind(tee_id)
    .fetch_all(&mut *connection)
    .await
}
