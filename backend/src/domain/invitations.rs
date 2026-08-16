use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};

use crate::domain::models::TournamentStatus;

const TOKEN_LENGTH: usize = 43;
const TOKEN_BYTES: usize = 32;
const MAX_NAME_BYTES: usize = 120;
const MAX_EMAIL_BYTES: usize = 254;

#[derive(Debug)]
pub struct RegistrationInput {
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub handicap_index: f64,
}

#[derive(Debug)]
pub struct ValidatedRegistration {
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub handicap_index: f64,
}

pub fn valid_token_shape(token: &str) -> bool {
    if token.len() != TOKEN_LENGTH {
        return false;
    }
    matches!(URL_SAFE_NO_PAD.decode(token), Ok(bytes) if bytes.len() == TOKEN_BYTES)
}

pub fn validate_registration(
    mut input: RegistrationInput,
) -> Result<ValidatedRegistration, &'static str> {
    input.email = input.email.trim().to_lowercase();
    validate_email(&input.email)?;
    if !(12..=128).contains(&input.password.len()) {
        return Err("account.password must be between 12 and 128 bytes");
    }
    if input.display_name.trim().is_empty()
        || input.display_name.len() > MAX_NAME_BYTES
        || input.display_name.contains('\0')
    {
        return Err("player.display_name is invalid");
    }
    if !input.handicap_index.is_finite() || !(-10.0..=54.0).contains(&input.handicap_index) {
        return Err("player.handicap_index must be between -10.0 and 54.0");
    }
    Ok(ValidatedRegistration {
        email: input.email,
        password: input.password,
        display_name: input.display_name.trim().to_owned(),
        handicap_index: input.handicap_index,
    })
}

pub fn validate_issue_policy(
    expires_at: DateTime<Utc>,
    max_uses: Option<i32>,
    now: DateTime<Utc>,
) -> Result<(), &'static str> {
    if expires_at <= now {
        return Err("expires_at must be in the future");
    }
    if matches!(max_uses, Some(value) if value <= 0) {
        return Err("max_uses must be null or a positive integer");
    }
    Ok(())
}

pub fn tournament_accepts_new_players(status: TournamentStatus) -> bool {
    matches!(status, TournamentStatus::Draft | TournamentStatus::Active)
}

fn validate_email(email: &str) -> Result<(), &'static str> {
    if email.is_empty()
        || email.len() > MAX_EMAIL_BYTES
        || email
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err("account.email must be a valid email of at most 254 bytes");
    }
    let Some((local, domain)) = email.split_once('@') else {
        return Err("account.email must be a valid email of at most 254 bytes");
    };
    if local.is_empty()
        || domain.is_empty()
        || domain.contains('@')
        || domain.starts_with('.')
        || domain.ends_with('.')
    {
        return Err("account.email must be a valid email of at most 254 bytes");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_shape_requires_one_256_bit_url_safe_value() {
        assert!(valid_token_shape(
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        ));
        assert!(!valid_token_shape("short"));
        assert!(!valid_token_shape(
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA!"
        ));
    }

    #[test]
    fn registration_normalizes_identity_and_rejects_invalid_values() {
        let valid = validate_registration(RegistrationInput {
            email: " PLAYER@Example.Test ".to_owned(),
            password: "a secure password".to_owned(),
            display_name: " Player ".to_owned(),
            handicap_index: 12.3,
        })
        .unwrap();
        assert_eq!(valid.email, "player@example.test");
        assert_eq!(valid.display_name, "Player");

        let invalid = RegistrationInput {
            email: "missing-at".to_owned(),
            password: "short".to_owned(),
            display_name: "".to_owned(),
            handicap_index: f64::NAN,
        };
        assert!(validate_registration(invalid).is_err());
    }
}
