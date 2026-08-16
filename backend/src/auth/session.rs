use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{TryRngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub const SESSION_COOKIE_NAME: &str = "golf_session";
const TOKEN_BYTES: usize = 32;

pub fn generate_session_token() -> Result<String, rand::rand_core::OsError> {
    generate_opaque_token()
}

pub fn generate_invitation_token() -> Result<String, rand::rand_core::OsError> {
    generate_opaque_token()
}

fn generate_opaque_token() -> Result<String, rand::rand_core::OsError> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    OsRng.try_fill_bytes(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn hash_session_token(token: &str) -> [u8; TOKEN_BYTES] {
    hash_opaque_token(token)
}

pub fn hash_invitation_token(token: &str) -> [u8; TOKEN_BYTES] {
    hash_opaque_token(token)
}

fn hash_opaque_token(token: &str) -> [u8; TOKEN_BYTES] {
    Sha256::digest(token.as_bytes()).into()
}

pub fn derive_csrf_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"golf-csrf-v1:");
    hasher.update(token.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

pub fn verify_csrf_token(token: &str, presented: &str) -> bool {
    let expected = derive_csrf_token(token);
    verify_derived_csrf(&expected, presented)
}

pub fn verify_derived_csrf(expected: &str, presented: &str) -> bool {
    expected.as_bytes().ct_eq(presented.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_256_bit_url_safe_values() {
        let first = generate_session_token().unwrap();
        let second = generate_session_token().unwrap();
        assert_ne!(first, second);
        assert_eq!(URL_SAFE_NO_PAD.decode(first).unwrap().len(), TOKEN_BYTES);
        let invitation = generate_invitation_token().unwrap();
        assert_eq!(
            URL_SAFE_NO_PAD.decode(invitation).unwrap().len(),
            TOKEN_BYTES
        );
    }

    #[test]
    fn csrf_tokens_are_session_bound() {
        let token = "session-token";
        let csrf = derive_csrf_token(token);
        assert!(verify_csrf_token(token, &csrf));
        assert!(!verify_csrf_token("other-session", &csrf));
    }
}
