use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Game {
    pub tournament_id: Uuid,
    pub enabled: bool,
    pub rules_version: i16,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Window {
    pub round_id: Uuid,
    pub deadline: Option<DateTime<Utc>>,
    pub opened_at: Option<DateTime<Utc>>,
    pub locked_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Save {
    pub request_id: Uuid,
    pub expected_revision: i64,
    pub picks: Vec<Uuid>,
    pub captain: Uuid,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Receipt {
    pub id: Uuid,
    pub round_id: Uuid,
    pub user_id: Uuid,
    pub revision: i64,
    pub picks: Vec<Uuid>,
    pub captain: Uuid,
    pub origin: String,
    pub source_round: Option<Uuid>,
    pub request_id: Option<Uuid>,
    pub expected_revision: Option<i64>,
    pub accepted_at: DateTime<Utc>,
}
impl Receipt {
    pub(super) fn lineup(
        &self,
    ) -> Result<crate::domain::fantasy::selections::Lineup, super::Error> {
        use crate::domain::fantasy::{GolferId, selections::Lineup};
        Lineup::new(
            &self.picks.iter().copied().map(GolferId).collect::<Vec<_>>(),
            GolferId(self.captain),
        )
        .map_err(|_| super::Error::Invalid)
    }
}
#[derive(Debug, Serialize)]
pub struct Selection {
    pub user_id: Uuid,
    pub state: String,
    pub locked_at: Option<DateTime<Utc>>,
    pub receipt: Option<Receipt>,
}
#[derive(Debug, Serialize)]
pub struct RoundView {
    pub selection_availability: SelectionAvailability,
    pub carry_forward_eligible: bool,
    #[serde(skip)]
    pub finalized: bool,
    pub window: Window,
    pub entered: bool,
    pub eligible_players: Vec<Uuid>,
    pub selections: Vec<Selection>,
    pub carry_forward_preview: Option<Receipt>,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    Player,
    Team,
}
impl OwnerKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Team => "team",
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Source {
    pub round_id: Uuid,
    pub owner_kind: OwnerKind,
    pub owner_id: Uuid,
    pub source_token: String,
    pub disposition: Option<Disposition>,
    pub disposition_current: bool,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Disposition {
    #[serde(skip)]
    pub(super) source_token: Vec<u8>,
    pub id: Uuid,
    pub disposed: bool,
    pub correction: bool,
    pub reason: String,
    pub actor_id: Uuid,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dispose {
    pub expected_source_token: String,
    pub disposed: bool,
    pub correction: bool,
    pub reason: String,
}
pub(super) const RECEIPT_COLUMNS: &str = "id,round_id,user_id,revision,ARRAY[first_player,second_player,third_player,fourth_player] AS picks,captain,origin,source_round,request_id,expected_revision,accepted_at";

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionAvailability {
    Open,
    Closed,
    NotEntered,
    InsufficientPlayers,
}
