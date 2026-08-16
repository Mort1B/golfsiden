pub const USERNAME_ERROR: &str =
    "username must contain 3 to 32 lowercase letters, digits, underscores, or hyphens";

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
}
