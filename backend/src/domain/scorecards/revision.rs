use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use uuid::Uuid;

/// A positive PostgreSQL bigint represented losslessly in every JSON client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreRevision(i64);
impl ScoreRevision {
    pub fn from_database(value: i64) -> Option<Self> {
        (value > 0).then_some(Self(value))
    }
    pub fn as_i64(self) -> i64 {
        self.0
    }
}
impl Serialize for ScoreRevision {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for ScoreRevision {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value.is_empty() || value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(D::Error::custom("invalid score revision"));
        }
        value
            .parse()
            .ok()
            .and_then(Self::from_database)
            .ok_or_else(|| D::Error::custom("invalid score revision"))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExpectedScore {
    Absent {},
    Present {
        score_id: Uuid,
        revision: ScoreRevision,
    },
}
impl ExpectedScore {
    pub fn matches(self, current: Option<&super::ScoreEntry>) -> bool {
        match (self, current) {
            (Self::Absent {}, None) => true,
            (Self::Present { score_id, revision }, Some(current)) => {
                score_id == current.id && revision == current.revision
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AppliedScore {
    pub score_id: Uuid,
    pub revision: ScoreRevision,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ScoreAcknowledgement {
    pub request_id: Uuid,
    pub applied_score: AppliedScore,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revision_is_canonical_positive_lossless_decimal_string() {
        for value in [1, 9_007_199_254_740_993, i64::MAX] {
            let revision = ScoreRevision::from_database(value).unwrap();
            let json = serde_json::to_string(&revision).unwrap();
            assert_eq!(json, format!("\"{value}\""));
            assert_eq!(
                serde_json::from_str::<ScoreRevision>(&json).unwrap(),
                revision
            );
        }
        for json in [
            "1",
            "null",
            "\"0\"",
            "\"01\"",
            "\"-1\"",
            "\"+1\"",
            "\"1e2\"",
            "\" 1\"",
            "\"9223372036854775808\"",
        ] {
            assert!(serde_json::from_str::<ScoreRevision>(json).is_err());
        }
    }
    #[test]
    fn expected_absence_and_present_versions_are_closed_variants() {
        assert!(
            serde_json::from_str::<ExpectedScore>(r#"{"type":"absent","revision":"1"}"#).is_err()
        );
        assert!(ExpectedScore::Absent {}.matches(None));
        assert!(
            !ExpectedScore::Present {
                score_id: Uuid::nil(),
                revision: ScoreRevision(1)
            }
            .matches(None)
        );
    }
}
