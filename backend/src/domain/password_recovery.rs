use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{TryRngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Deliberately has no Debug or Serialize implementation.
pub struct RecoveryToken(String);
pub struct RecoveryTokenHash([u8; 32]);

impl RecoveryToken {
    pub fn generate() -> Result<Self, rand::rand_core::OsError> {
        let mut bytes = [0_u8; 32];
        OsRng.try_fill_bytes(&mut bytes)?;
        Ok(Self(URL_SAFE_NO_PAD.encode(bytes)))
    }
    pub fn parse(value: String) -> Option<Self> {
        (value.len() == 43
            && matches!(URL_SAFE_NO_PAD.decode(&value), Ok(bytes) if bytes.len()==32))
        .then_some(Self(value))
    }
    pub fn hash(&self) -> RecoveryTokenHash {
        RecoveryTokenHash(Sha256::digest(self.0.as_bytes()).into())
    }
    pub fn matches(&self, stored: &[u8]) -> bool {
        self.hash().as_bytes().ct_eq(stored).into()
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl RecoveryTokenHash {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_are_independent_256_bit_secrets_with_exact_hashes() {
        let a = RecoveryToken::generate().unwrap();
        let b = RecoveryToken::generate().unwrap();
        assert_ne!(a.expose(), b.expose());
        assert_eq!(a.expose().len(), 43);
        assert!(a.matches(a.hash().as_bytes()));
        assert!(!a.matches(b.hash().as_bytes()));
        assert!(RecoveryToken::parse(a.expose().to_owned()).is_some());
        assert!(RecoveryToken::parse("invalid".into()).is_none());
    }
}
