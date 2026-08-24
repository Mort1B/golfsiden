use chrono::{DateTime, Days, NaiveDate, Utc};

use crate::domain::{
    accounts::normalize_and_validate_username,
    models::{ScoringFormat, ScoringMode},
};

const MAX_NAME_BYTES: usize = 120;
const MAX_DESCRIPTION_BYTES: usize = 2_000;

#[derive(Debug)]
pub struct OnboardingInput {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub handicap_index: f64,
    pub tournament_name: String,
    pub description: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub rounds: Vec<RoundInput>,
}

#[derive(Debug)]
pub struct RoundInput {
    pub round_number: i16,
    pub name: String,
    pub round_date: NaiveDate,
    pub scoring_format: ScoringFormat,
}

#[derive(Debug)]
pub struct ValidatedOnboarding {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub handicap_index: f64,
    pub tournament_name: String,
    pub description: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub scoring_mode: ScoringMode,
    pub invitation_expires_at: DateTime<Utc>,
    pub rounds: Vec<ValidatedRound>,
}

#[derive(Debug)]
pub struct ValidatedRound {
    pub round_number: i16,
    pub name: String,
    pub round_date: NaiveDate,
    pub scoring_format: ScoringFormat,
}

pub fn validate(
    mut input: OnboardingInput,
    today: NaiveDate,
) -> Result<ValidatedOnboarding, &'static str> {
    input.username = normalize_and_validate_username(&input.username)
        .map_err(|_| "creator.account.username is invalid")?;
    if !(12..=128).contains(&input.password.len()) {
        return Err("creator.account.password must be between 12 and 128 bytes");
    }
    validate_name(
        &input.display_name,
        "creator.player.display_name is invalid",
    )?;
    if !input.handicap_index.is_finite() || !(-10.0..=54.0).contains(&input.handicap_index) {
        return Err("creator.player.handicap_index must be between -10.0 and 54.0");
    }
    validate_name(&input.tournament_name, "tournament.name is invalid")?;
    if input.description.len() > MAX_DESCRIPTION_BYTES || input.description.contains('\0') {
        return Err("tournament.description must not exceed 2000 bytes");
    }
    if input.end_date < input.start_date {
        return Err("tournament.end_date must not be before start_date");
    }
    if input.end_date < today {
        return Err("tournament.end_date must not be in the past");
    }
    if !(1..=30).contains(&input.rounds.len()) {
        return Err("rounds must contain between 1 and 30 entries");
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

    Ok(ValidatedOnboarding {
        username: input.username,
        password: input.password,
        display_name: input.display_name.trim().to_owned(),
        handicap_index: input.handicap_index,
        tournament_name: input.tournament_name.trim().to_owned(),
        description: input.description.trim().to_owned(),
        start_date: input.start_date,
        end_date: input.end_date,
        scoring_mode,
        invitation_expires_at: DateTime::from_naive_utc_and_offset(expiry_naive, Utc),
        rounds,
    })
}

fn validate_name(value: &str, message: &'static str) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > MAX_NAME_BYTES || value.contains('\0') {
        return Err(message);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input() -> OnboardingInput {
        OnboardingInput {
            username: " Creator_1 ".to_owned(),
            password: "long-test-password".to_owned(),
            display_name: " Creator ".to_owned(),
            handicap_index: 12.3,
            tournament_name: " Trip ".to_owned(),
            description: " Annual trip ".to_owned(),
            start_date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 9, 3).unwrap(),
            rounds: vec![
                RoundInput {
                    round_number: 2,
                    name: "Scramble".to_owned(),
                    round_date: NaiveDate::from_ymd_opt(2026, 9, 2).unwrap(),
                    scoring_format: ScoringFormat::TeamScramble,
                },
                RoundInput {
                    round_number: 1,
                    name: "Opening".to_owned(),
                    round_date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                    scoring_format: ScoringFormat::IndividualStrokePlay,
                },
            ],
        }
    }

    #[test]
    fn normalizes_and_derives_combined_round_plan() {
        let validated =
            validate(valid_input(), NaiveDate::from_ymd_opt(2026, 8, 16).unwrap()).unwrap();
        assert_eq!(validated.username, "creator_1");
        assert_eq!(validated.scoring_mode, ScoringMode::Combined);
        assert_eq!(validated.rounds[0].round_number, 1);
        assert_eq!(
            validated.invitation_expires_at.to_rfc3339(),
            "2026-09-10T00:00:00+00:00"
        );
    }

    #[test]
    fn rejects_noncontiguous_rounds_and_past_end_date() {
        let mut input = valid_input();
        input.rounds[0].round_number = 3;
        assert!(validate(input, NaiveDate::from_ymd_opt(2026, 8, 16).unwrap()).is_err());

        let mut input = valid_input();
        input.end_date = NaiveDate::from_ymd_opt(2026, 8, 15).unwrap();
        assert!(validate(input, NaiveDate::from_ymd_opt(2026, 8, 16).unwrap()).is_err());
    }

    #[test]
    fn derives_single_format_summary_modes() {
        let mut individual = valid_input();
        individual.rounds[0].scoring_format = ScoringFormat::IndividualStrokePlay;
        assert_eq!(
            validate(individual, NaiveDate::from_ymd_opt(2026, 8, 16).unwrap())
                .unwrap()
                .scoring_mode,
            ScoringMode::Individual
        );

        let mut team = valid_input();
        team.rounds[1].scoring_format = ScoringFormat::TeamScramble;
        assert_eq!(
            validate(team, NaiveDate::from_ymd_opt(2026, 8, 16).unwrap())
                .unwrap()
                .scoring_mode,
            ScoringMode::Team
        );
    }
}
