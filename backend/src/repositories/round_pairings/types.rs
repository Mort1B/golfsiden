use chrono::{DateTime, NaiveTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::models::{ParticipantStatus, RoundStatus, ScoringFormat};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PairingEntrant {
    pub player_id: Uuid,
    pub display_name: String,
    pub status: ParticipantStatus,
    pub player_active: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PairingMember {
    pub player_id: Uuid,
    pub display_name: String,
    pub display_order: Option<i16>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PairingGroup {
    pub id: Uuid,
    pub name: String,
    pub starting_hole: Option<i16>,
    pub tee_time: Option<NaiveTime>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(skip)]
    pub members: Vec<PairingMember>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoundPairings {
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub status: RoundStatus,
    pub scoring_format: ScoringFormat,
    pub updated_at: DateTime<Utc>,
    pub active_entrants: Vec<PairingEntrant>,
    pub inactive_entrants: Vec<PairingEntrant>,
    pub teams: Vec<PairingGroup>,
    pub flights: Vec<PairingGroup>,
    pub legacy_individual_groups: Vec<PairingGroup>,
}

#[derive(Debug, FromRow)]
pub(super) struct RoundPairingRow {
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub status: RoundStatus,
    pub scoring_format: ScoringFormat,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub(super) struct StoredGroup {
    pub id: Uuid,
    pub name: String,
    pub starting_hole: Option<i16>,
    pub tee_time: Option<NaiveTime>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub(super) struct LegacyFacts {
    pub by_flight: HashMap<Uuid, StoredGroup>,
    pub members_by_flight: HashMap<Uuid, HashMap<Uuid, LegacyMemberFact>>,
}

#[derive(Clone, Copy, sqlx::FromRow)]
pub(super) struct LegacyMemberFact {
    pub player_id: Uuid,
    pub display_order: Option<i16>,
    pub created_at: DateTime<Utc>,
}
