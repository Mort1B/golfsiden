pub use crate::domain::tournament_plan::RoundInput;
use crate::domain::{
    accounts::{normalize_and_validate_username, validate_password_length},
    tournament_plan::{self, TournamentPlanInput, ValidatedTournamentPlan, validate_name},
};
use chrono::NaiveDate;

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
    pub counted_rounds: i16,
    pub mandatory_round_number: Option<i16>,
    pub rounds: Vec<RoundInput>,
}

#[derive(Debug)]
pub struct ValidatedOnboarding {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub handicap_index: f64,
    pub plan: ValidatedTournamentPlan,
}
pub fn validate(
    input: OnboardingInput,
    today: NaiveDate,
) -> Result<ValidatedOnboarding, &'static str> {
    let username = normalize_and_validate_username(&input.username)
        .map_err(|_| "creator.account.username is invalid")?;
    if validate_password_length(&input.password).is_err() {
        return Err("creator.account.password must be between 12 and 128 bytes");
    }
    validate_name(
        &input.display_name,
        "creator.player.display_name is invalid",
    )?;
    if !input.handicap_index.is_finite() || !(-10.0..=54.0).contains(&input.handicap_index) {
        return Err("creator.player.handicap_index must be between -10.0 and 54.0");
    }
    let plan = tournament_plan::validate(
        TournamentPlanInput {
            tournament_name: input.tournament_name,
            description: input.description,
            start_date: input.start_date,
            end_date: input.end_date,
            counted_rounds: input.counted_rounds,
            mandatory_round_number: input.mandatory_round_number,
            rounds: input.rounds,
        },
        today,
    )?;
    Ok(ValidatedOnboarding {
        username,
        password: input.password,
        display_name: input.display_name.trim().to_owned(),
        handicap_index: input.handicap_index,
        plan,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{ScoringFormat, ScoringMode};

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
            counted_rounds: 2,
            mandatory_round_number: None,
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
        assert_eq!(validated.plan.scoring_mode, ScoringMode::Combined);
        assert_eq!(validated.plan.counted_rounds, 2);
        assert_eq!(validated.plan.mandatory_round_number, None);
        assert_eq!(validated.plan.rounds[0].round_number, 1);
        assert_eq!(
            validated.plan.invitation_expires_at.to_rfc3339(),
            "2026-09-10T00:00:00+00:00"
        );
    }

    #[test]
    fn rejects_noncontiguous_rounds_and_past_end_date() {
        let mut input = valid_input();
        input.rounds[0].round_number = 3;
        assert!(validate(input, NaiveDate::from_ymd_opt(2026, 8, 16).unwrap()).is_err());

        let mut input = valid_input();
        input.counted_rounds = 3;
        assert!(validate(input, NaiveDate::from_ymd_opt(2026, 8, 16).unwrap()).is_err());

        let mut input = valid_input();
        input.mandatory_round_number = Some(3);
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
                .plan
                .scoring_mode,
            ScoringMode::Individual
        );

        let mut team = valid_input();
        team.rounds[1].scoring_format = ScoringFormat::TeamScramble;
        assert_eq!(
            validate(team, NaiveDate::from_ymd_opt(2026, 8, 16).unwrap())
                .unwrap()
                .plan
                .scoring_mode,
            ScoringMode::Team
        );
    }
}
