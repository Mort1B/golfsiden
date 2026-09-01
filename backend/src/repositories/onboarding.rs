use chrono::{DateTime, Utc};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    auth::SessionPrincipal,
    domain::{
        models::{Round, Tournament, TournamentRole},
        onboarding::ValidatedOnboarding,
    },
    repositories::auth,
};

const TOURNAMENT_COLUMNS: &str = "id, name, description, start_date, end_date, number_of_rounds, counted_rounds, mandatory_round_id, status, scoring_mode, created_at, updated_at";
const ROUND_COLUMNS: &str = "id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format, created_at, updated_at";

#[derive(Debug, Error)]
pub enum OnboardingRepositoryError {
    #[error("username is already registered")]
    DuplicateUsername,
    #[error("database operation failed")]
    Database(#[source] sqlx::Error),
}

#[derive(Debug)]
pub struct CreatedOnboarding {
    pub tournament: Tournament,
    pub rounds: Vec<Round>,
    pub session: SessionPrincipal,
    pub creator_player_id: Uuid,
    pub creator_role: TournamentRole,
    pub invitation_id: Uuid,
}

pub struct CreateOnboardingParams<'a> {
    pub input: &'a ValidatedOnboarding,
    pub password_hash: &'a str,
    pub session_token_hash: &'a [u8],
    pub session_expires_at: DateTime<Utc>,
    pub invitation_token_hash: &'a [u8],
}

pub async fn create(
    pool: &PgPool,
    params: CreateOnboardingParams<'_>,
) -> Result<CreatedOnboarding, OnboardingRepositoryError> {
    let input = params.input;
    let mut transaction = pool.begin().await.map_err(classify_error)?;
    let player_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO players (id, display_name, current_handicap_index)
         VALUES ($1, $2, $3)",
    )
    .bind(player_id)
    .bind(&input.display_name)
    .bind(input.handicap_index)
    .execute(&mut *transaction)
    .await
    .map_err(classify_error)?;

    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, username, display_name, role, password_hash, player_id)
         VALUES ($1, $2, $3, 'player', $4, $5)",
    )
    .bind(user_id)
    .bind(&input.username)
    .bind(&input.display_name)
    .bind(params.password_hash)
    .bind(player_id)
    .execute(&mut *transaction)
    .await
    .map_err(classify_error)?;

    sqlx::query(
        "INSERT INTO handicap_history
           (id, player_id, handicap_index, changed_by, reason)
         VALUES ($1, $2, $3, $4, 'initial account handicap')",
    )
    .bind(Uuid::new_v4())
    .bind(player_id)
    .bind(input.handicap_index)
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    .map_err(classify_error)?;

    let tournament_id = Uuid::new_v4();
    let round_ids = input
        .rounds
        .iter()
        .map(|round| (round.round_number, Uuid::new_v4()))
        .collect::<Vec<_>>();
    let mandatory_round_id = match input.mandatory_round_number {
        Some(required) => Some(
            round_ids
                .iter()
                .find_map(|(number, id)| (*number == required).then_some(*id))
                .ok_or_else(|| {
                    OnboardingRepositoryError::Database(sqlx::Error::Protocol(
                        "validated mandatory round was absent from the round plan".to_owned(),
                    ))
                })?,
        ),
        None => None,
    };
    let tournament = sqlx::query_as::<_, Tournament>(&format!(
        "INSERT INTO tournaments
           (id, name, description, start_date, end_date, number_of_rounds, counted_rounds, mandatory_round_id, status, scoring_mode)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'draft', $9)
         RETURNING {TOURNAMENT_COLUMNS}"
    ))
    .bind(tournament_id)
    .bind(&input.tournament_name)
    .bind(&input.description)
    .bind(input.start_date)
    .bind(input.end_date)
    .bind(i16::try_from(input.rounds.len()).map_err(|_| {
        OnboardingRepositoryError::Database(sqlx::Error::Protocol(
            "validated round count exceeded i16".to_owned(),
        ))
    })?)
    .bind(input.counted_rounds)
    .bind(mandatory_round_id)
    .bind(input.scoring_mode)
    .fetch_one(&mut *transaction)
    .await
    .map_err(classify_error)?;

    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin')",
    )
    .bind(tournament_id)
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    .map_err(classify_error)?;
    sqlx::query(
        "INSERT INTO tournament_players
           (tournament_id, player_id, tournament_handicap)
         VALUES ($1, $2, $3)",
    )
    .bind(tournament_id)
    .bind(player_id)
    .bind(input.handicap_index)
    .execute(&mut *transaction)
    .await
    .map_err(classify_error)?;
    sqlx::query(
        "INSERT INTO tournament_handicap_history
           (id, tournament_id, player_id, handicap_index, changed_by, reason)
         VALUES ($1, $2, $3, $4, $5, 'initial tournament handicap')",
    )
    .bind(Uuid::new_v4())
    .bind(tournament_id)
    .bind(player_id)
    .bind(input.handicap_index)
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    .map_err(classify_error)?;

    let mut rounds = Vec::with_capacity(input.rounds.len());
    for (input_round, (_, round_id)) in input.rounds.iter().zip(round_ids) {
        let allowance =
            crate::domain::round_formats::RoundFormatPolicy::for_format(input_round.scoring_format)
                .required_allowance_percent()
                .unwrap_or(100);
        let round = sqlx::query_as::<_, Round>(&format!(
            "INSERT INTO rounds
               (id, tournament_id, round_number, name, round_date,
                course_id, course_name, tee_id, tee_name, number_of_holes,
                status, handicap_enabled, handicap_allowance_percent, scoring_format)
             VALUES ($1, $2, $3, $4, $5, NULL, '', NULL, '', 18,
                     'draft', TRUE, $6, $7)
             RETURNING {ROUND_COLUMNS}"
        ))
        .bind(round_id)
        .bind(tournament_id)
        .bind(input_round.round_number)
        .bind(&input_round.name)
        .bind(input_round.round_date)
        .bind(allowance)
        .bind(input_round.scoring_format)
        .fetch_one(&mut *transaction)
        .await
        .map_err(classify_error)?;
        rounds.push(round);
    }

    let invitation_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at, max_uses, series_id)
         VALUES ($1, $2, $3, $4, $5, NULL, $1)",
    )
    .bind(invitation_id)
    .bind(tournament_id)
    .bind(params.invitation_token_hash)
    .bind(user_id)
    .bind(input.invitation_expires_at)
    .execute(&mut *transaction)
    .await
    .map_err(classify_error)?;

    let session = auth::create_session_in_transaction(
        &mut transaction,
        user_id,
        params.session_token_hash,
        params.session_expires_at,
    )
    .await
    .map_err(classify_error)?;
    transaction.commit().await.map_err(classify_error)?;

    Ok(CreatedOnboarding {
        tournament,
        rounds,
        session,
        creator_player_id: player_id,
        creator_role: TournamentRole::Admin,
        invitation_id,
    })
}

fn classify_error(error: sqlx::Error) -> OnboardingRepositoryError {
    if let sqlx::Error::Database(database) = &error
        && database.is_unique_violation()
        && matches!(database.constraint(), Some("users_username_normalized_idx"))
    {
        return OnboardingRepositoryError::DuplicateUsername;
    }
    OnboardingRepositoryError::Database(error)
}
