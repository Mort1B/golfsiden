use crate::domain::models::{ScoringFormat, ScoringMode};
use chrono::{DateTime, Days, NaiveDate, Utc};
use serde::Serialize;

const MAX_DESCRIPTION_BYTES: usize = 2_000;

#[derive(Debug)]
pub struct TournamentPlanInput {
    pub tournament_name: String,
    pub description: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub counted_rounds: i16,
    pub mandatory_round_number: Option<i16>,
    pub rounds: Vec<RoundInput>,
}

#[derive(Debug)]
pub struct RoundInput {
    pub round_number: i16,
    pub name: String,
    pub round_date: NaiveDate,
    pub scoring_format: ScoringFormat,
}

#[derive(Debug, Serialize)]
pub struct ValidatedTournamentPlan {
    pub tournament_name: String,
    pub description: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub counted_rounds: i16,
    pub mandatory_round_number: Option<i16>,
    pub scoring_mode: ScoringMode,
    pub invitation_expires_at: DateTime<Utc>,
    pub rounds: Vec<ValidatedRound>,
}

#[derive(Debug, Serialize)]
pub struct ValidatedRound {
    pub round_number: i16,
    pub name: String,
    pub round_date: NaiveDate,
    pub scoring_format: ScoringFormat,
}

pub fn validate(
    input: TournamentPlanInput,
    today: NaiveDate,
) -> Result<ValidatedTournamentPlan, &'static str> {
    let plan = normalize(input)?;
    if plan.end_date < today {
        return Err("tournament.end_date must not be in the past");
    }
    Ok(plan)
}

// Stable validation for idempotent receipts; only a new creation checks today.
pub fn normalize(mut input: TournamentPlanInput) -> Result<ValidatedTournamentPlan, &'static str> {
    validate_name(&input.tournament_name, "tournament.name is invalid")?;
    if input.description.len() > MAX_DESCRIPTION_BYTES || input.description.contains('\0') {
        return Err("tournament.description must not exceed 2000 bytes");
    }
    if input.end_date < input.start_date {
        return Err("tournament.end_date must not be before start_date");
    }
    if !(1..=30).contains(&input.rounds.len()) {
        return Err("rounds must contain between 1 and 30 entries");
    }
    let round_count = i16::try_from(input.rounds.len()).map_err(|_| "too many rounds")?;
    if input.counted_rounds < 1 || input.counted_rounds > round_count {
        return Err("tournament.counted_rounds must be between 1 and the round count");
    }
    if input
        .mandatory_round_number
        .is_some_and(|round_number| !(1..=round_count).contains(&round_number))
    {
        return Err("tournament.mandatory_round_number must identify a configured round");
    }

    input.rounds.sort_by_key(|round| round.round_number);
    for (index, round) in input.rounds.iter().enumerate() {
        let expected = i16::try_from(index + 1).map_err(|_| "too many rounds")?;
        if round.round_number != expected {
            return Err("rounds must have unique contiguous round_number values starting at 1");
        }
        validate_name(&round.name, "round name is invalid")?;
        if round.round_date < input.start_date || round.round_date > input.end_date {
            return Err("every round_date must be within the tournament date range");
        }
    }

    let has_individual = input
        .rounds
        .iter()
        .any(|round| round.scoring_format == ScoringFormat::IndividualStrokePlay);
    let has_team = input.rounds.iter().any(|round| {
        crate::domain::round_formats::RoundFormatPolicy::for_format(round.scoring_format)
            .owner_kind()
            == crate::domain::round_formats::ScoreOwnerKind::Team
    });
    let scoring_mode = match (has_individual, has_team) {
        (true, false) => ScoringMode::Individual,
        (false, true) => ScoringMode::Team,
        (true, true) => ScoringMode::Combined,
        (false, false) => return Err("rounds must not be empty"),
    };
    let expiry_date = input
        .end_date
        .checked_add_days(Days::new(7))
        .ok_or("tournament.end_date is too late")?;
    let expiry_naive = expiry_date
        .and_hms_opt(0, 0, 0)
        .ok_or("tournament.end_date is invalid")?;
    let rounds = input
        .rounds
        .into_iter()
        .map(|round| ValidatedRound {
            round_number: round.round_number,
            name: round.name.trim().to_owned(),
            round_date: round.round_date,
            scoring_format: round.scoring_format,
        })
        .collect();

    Ok(ValidatedTournamentPlan {
        tournament_name: input.tournament_name.trim().to_owned(),
        description: input.description.trim().to_owned(),
        start_date: input.start_date,
        end_date: input.end_date,
        counted_rounds: input.counted_rounds,
        mandatory_round_number: input.mandatory_round_number,
        scoring_mode,
        invitation_expires_at: DateTime::from_naive_utc_and_offset(expiry_naive, Utc),
        rounds,
    })
}

pub(crate) fn validate_name(value: &str, message: &'static str) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > 120 || value.contains('\0') {
        return Err(message);
    }
    Ok(())
}
