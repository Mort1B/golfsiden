use crate::domain::{
    models::{Round, Tournament},
    tournament_plan::ValidatedTournamentPlan,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

const TOURNAMENT_COLUMNS: &str = "id, name, description, start_date, end_date, number_of_rounds, counted_rounds, mandatory_round_id, status, scoring_mode, created_at, updated_at";
const ROUND_COLUMNS: &str = "id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format, created_at, updated_at";

pub async fn insert(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    entrant: Option<(Uuid, f64)>,
    input: &ValidatedTournamentPlan,
) -> Result<(Tournament, Vec<Round>), sqlx::Error> {
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
                    sqlx::Error::Protocol(
                        "validated mandatory round was absent from the round plan".to_owned(),
                    )
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
        sqlx::Error::Protocol("validated round count exceeded i16".to_owned())
    })?)
    .bind(input.counted_rounds)
    .bind(mandatory_round_id)
    .bind(input.scoring_mode)
    .fetch_one(&mut **transaction)
    .await?;

    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin')",
    )
    .bind(tournament_id)
    .bind(user_id)
    .execute(&mut **transaction)
    .await?;
    if let Some((player_id, handicap)) = entrant {
        sqlx::query(
            "INSERT INTO tournament_players
           (tournament_id, player_id, tournament_handicap)
         VALUES ($1, $2, $3)",
        )
        .bind(tournament_id)
        .bind(player_id)
        .bind(handicap)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            "INSERT INTO tournament_handicap_history
           (id, tournament_id, player_id, handicap_index, changed_by, reason)
         VALUES ($1, $2, $3, $4, $5, 'initial tournament handicap')",
        )
        .bind(Uuid::new_v4())
        .bind(tournament_id)
        .bind(player_id)
        .bind(handicap)
        .bind(user_id)
        .execute(&mut **transaction)
        .await?;
    }

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
        .fetch_one(&mut **transaction)
        .await?;
        rounds.push(round);
    }

    Ok((tournament, rounds))
}
