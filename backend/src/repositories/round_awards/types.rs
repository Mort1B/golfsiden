use crate::domain::round_awards::AwardType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub version: u8,
    pub request_id: Uuid,
    pub award_type: AwardType,
    #[serde(deserialize_with = "Option::<Uuid>::deserialize")]
    pub player_id: Option<Uuid>,
    pub expected_revision: String,
    pub reason: String,
    pub correction: bool,
}
impl Command {
    pub(super) fn revision(&self) -> Option<i64> {
        let n = self.expected_revision.parse::<i64>().ok()?;
        (self.version == 1
            && (0..i64::MAX).contains(&n)
            && n.to_string() == self.expected_revision
            && !self.reason.trim().is_empty()
            && self.reason.len() <= 500
            && !self.reason.contains('\0')
            && !self.request_id.is_nil())
        .then_some(n)
    }
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Receipt {
    pub request_id: Uuid,
    pub round_id: Uuid,
    pub award_type: AwardType,
    pub player_id: Option<Uuid>,
    pub previous_player_id: Option<Uuid>,
    pub revision: String,
    pub reason: String,
    pub correction: bool,
    pub actor_id: Uuid,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Player {
    pub player_id: Uuid,
    pub display_name: String,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Slot {
    pub award_type: AwardType,
    pub player_id: Option<Uuid>,
    pub revision: String,
}
#[derive(Debug, Serialize)]
pub struct View {
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub availability: &'static str,
    pub correction_required: bool,
    pub eligible_players: Vec<Player>,
    pub awards: Vec<Slot>,
    pub history: Vec<Receipt>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removal_is_explicit_and_revision_is_canonical() {
        let value = serde_json::json!({"version":1,"request_id":Uuid::new_v4(),"award_type":"closest_to_pin","player_id":null,"expected_revision":"0","reason":"Measured","correction":true});
        let command: Command = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(command.revision(), Some(0));
        let mut missing = value;
        missing.as_object_mut().unwrap().remove("player_id");
        assert!(serde_json::from_value::<Command>(missing).is_err());
        for revision in [
            "-1",
            "01",
            "1.0",
            "9223372036854775807",
            "9223372036854775808",
        ] {
            let mut invalid = command.clone();
            invalid.expected_revision = revision.into();
            assert!(invalid.revision().is_none());
        }
    }
}
