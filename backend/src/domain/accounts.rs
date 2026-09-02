pub const USERNAME_ERROR: &str =
    "username must contain 3 to 32 lowercase letters, digits, underscores, or hyphens";
pub const PASSWORD_ERROR: &str = "password must be between 12 and 128 bytes";

pub fn normalize_username(username: &str) -> String {
    username.trim().to_ascii_lowercase()
}

pub fn normalize_and_validate_username(username: &str) -> Result<String, &'static str> {
    let username = normalize_username(username);
    if (3..=32).contains(&username.len())
        && username
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte))
    {
        Ok(username)
    } else {
        Err(USERNAME_ERROR)
    }
}

pub fn validate_password_length(password: &str) -> Result<(), &'static str> {
    if (12..=128).contains(&password.len()) {
        Ok(())
    } else {
        Err(PASSWORD_ERROR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_ascii_usernames() {
        assert_eq!(
            normalize_and_validate_username(" Player_1 ").unwrap(),
            "player_1"
        );
    }

    #[test]
    fn rejects_out_of_contract_usernames() {
        for value in ["ab", "a.b", "spilleræ", "a username", &"a".repeat(33)] {
            assert!(normalize_and_validate_username(value).is_err());
        }
    }

    #[test]
    fn password_length_uses_account_contract_bytes() {
        assert!(validate_password_length("123456789012").is_ok());
        assert!(validate_password_length("short").is_err());
        assert!(validate_password_length(&"a".repeat(129)).is_err());
    }
}
