use crate::domain::password_recovery::RecoveryToken;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RecoveryOrigin(String);
impl RecoveryOrigin {
    pub fn parse(value: &str, development: bool) -> Result<Self, &'static str> {
        let uri: axum::http::Uri = value.parse().map_err(|_| "invalid reset origin")?;
        let scheme = uri.scheme_str().ok_or("invalid reset origin")?;
        let authority = uri.authority().ok_or("invalid reset origin")?;
        let localhost = matches!(uri.host(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if authority.as_str().contains('@')
            || value != format!("{scheme}://{authority}")
            || !(scheme == "https" || development && scheme == "http" && localhost)
        {
            return Err("reset origin requires HTTPS or development loopback HTTP");
        }
        Ok(Self(value.to_owned()))
    }
    pub fn link(&self, id: Uuid, token: &RecoveryToken) -> String {
        format!("{}/reset-password/{id}#token={}", self.0, token.expose())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origin_is_explicit_exact_and_secure() {
        for bad in [
            "http://example.test",
            "https://x/path",
            "https://x/",
            "https://x#y",
            "https://u@x",
            "https://x?y",
            "//x",
        ] {
            assert!(RecoveryOrigin::parse(bad, true).is_err(), "{bad}");
        }
        for origin in [
            "http://localhost:5173",
            "http://127.0.0.1:5173",
            "http://[::1]:5173",
        ] {
            assert!(RecoveryOrigin::parse(origin, true).is_ok());
            assert!(RecoveryOrigin::parse(origin, false).is_err());
        }
        assert!(RecoveryOrigin::parse("https://golf.example", false).is_ok());
    }
}
