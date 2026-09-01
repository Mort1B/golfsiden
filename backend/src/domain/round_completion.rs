use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::{
    models::RoundStatus,
    score_visibility::{VisibilityMetadata, VisibilityMode},
    scorecards::ScoreOwner,
};

#[derive(Debug, Clone)]
pub struct OwnerProgressFact {
    pub owner: ScoreOwner,
    pub owner_name: String,
    pub holes_scored: i64,
    pub confirmed: bool,
}

#[derive(Debug, Clone)]
pub struct CompletionFacts {
    pub round_id: Uuid,
    pub status: RoundStatus,
    pub required_holes: i16,
    pub owners: Vec<OwnerProgressFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnerCompletionProgress {
    pub owner: ScoreOwner,
    pub owner_name: String,
    pub holes_scored: i64,
    pub required_holes: i16,
    pub complete: bool,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionIssueCode {
    NoRequiredOwners,
    IncompleteScorecards,
    UnconfirmedScorecards,
    RoundNotOpen,
    RoundNotCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompletionIssue {
    pub code: CompletionIssueCode,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoundCompletionValidation {
    pub round_id: Uuid,
    pub status: RoundStatus,
    pub owners: Vec<OwnerCompletionProgress>,
    pub ready_to_complete: bool,
    pub ready_to_lock: bool,
    pub issues: Vec<CompletionIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnerCompletionReadProgress {
    pub owner: ScoreOwner,
    pub owner_name: String,
    pub holes_scored: i64,
    pub required_holes: i16,
    pub complete: Option<bool>,
    pub confirmed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoundCompletionReadProjection {
    pub round_id: Uuid,
    pub status: RoundStatus,
    pub owners: Vec<OwnerCompletionReadProgress>,
    pub ready_to_complete: Option<bool>,
    pub ready_to_lock: Option<bool>,
    pub issues: Vec<CompletionIssue>,
    pub visibility: VisibilityMetadata,
}

pub fn read_projection(
    validation: RoundCompletionValidation,
    visibility: VisibilityMetadata,
    visible_holes: &HashMap<ScoreOwner, i64>,
) -> RoundCompletionReadProjection {
    let restricted = visibility.mode == VisibilityMode::FrontNine;
    let owners = validation
        .owners
        .into_iter()
        .map(|owner| OwnerCompletionReadProgress {
            holes_scored: if restricted {
                visible_holes.get(&owner.owner).copied().unwrap_or_default()
            } else {
                owner.holes_scored
            },
            required_holes: if restricted { 9 } else { owner.required_holes },
            complete: (!restricted).then_some(owner.complete),
            confirmed: (!restricted).then_some(owner.confirmed),
            owner: owner.owner,
            owner_name: owner.owner_name,
        })
        .collect();
    RoundCompletionReadProjection {
        round_id: validation.round_id,
        status: validation.status,
        owners,
        ready_to_complete: (!restricted).then_some(validation.ready_to_complete),
        ready_to_lock: (!restricted).then_some(validation.ready_to_lock),
        issues: if restricted {
            Vec::new()
        } else {
            validation.issues
        },
        visibility,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionAction {
    Complete,
    Lock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionBlocker {
    InvalidSourceState,
    NoRequiredOwners,
    IncompleteScorecards,
    UnconfirmedScorecards,
}

pub fn validate(mut facts: CompletionFacts) -> RoundCompletionValidation {
    facts.owners.sort_by(|left, right| {
        left.owner_name
            .cmp(&right.owner_name)
            .then(owner_id(left.owner).cmp(&owner_id(right.owner)))
    });
    let owners = facts
        .owners
        .into_iter()
        .map(|owner| OwnerCompletionProgress {
            owner: owner.owner,
            owner_name: owner.owner_name,
            holes_scored: owner.holes_scored,
            required_holes: facts.required_holes,
            complete: owner.holes_scored == i64::from(facts.required_holes),
            confirmed: owner.confirmed,
        })
        .collect::<Vec<_>>();
    let has_owners = !owners.is_empty();
    let all_complete = has_owners && owners.iter().all(|owner| owner.complete);
    let all_confirmed = has_owners && owners.iter().all(|owner| owner.confirmed);
    let scorecards_ready = all_complete && all_confirmed;
    let mut issues = Vec::new();
    push_issue(
        &mut issues,
        !has_owners,
        CompletionIssueCode::NoRequiredOwners,
        "round has no required scorecards",
    );
    push_issue(
        &mut issues,
        has_owners && !all_complete,
        CompletionIssueCode::IncompleteScorecards,
        "one or more scorecards are incomplete",
    );
    push_issue(
        &mut issues,
        has_owners && !all_confirmed,
        CompletionIssueCode::UnconfirmedScorecards,
        "one or more scorecards are unconfirmed",
    );
    push_issue(
        &mut issues,
        facts.status != RoundStatus::Open,
        CompletionIssueCode::RoundNotOpen,
        "round must be open to complete",
    );
    push_issue(
        &mut issues,
        facts.status != RoundStatus::Completed,
        CompletionIssueCode::RoundNotCompleted,
        "round must be completed to lock",
    );
    RoundCompletionValidation {
        round_id: facts.round_id,
        status: facts.status,
        owners,
        ready_to_complete: facts.status == RoundStatus::Open && scorecards_ready,
        ready_to_lock: facts.status == RoundStatus::Completed && scorecards_ready,
        issues,
    }
}

pub fn transition_blocker(
    validation: &RoundCompletionValidation,
    action: TransitionAction,
) -> Option<TransitionBlocker> {
    let expected_status = match action {
        TransitionAction::Complete => RoundStatus::Open,
        TransitionAction::Lock => RoundStatus::Completed,
    };
    if validation.status != expected_status {
        return Some(TransitionBlocker::InvalidSourceState);
    }
    if validation.owners.is_empty() {
        return Some(TransitionBlocker::NoRequiredOwners);
    }
    if validation.owners.iter().any(|owner| !owner.complete) {
        return Some(TransitionBlocker::IncompleteScorecards);
    }
    if validation.owners.iter().any(|owner| !owner.confirmed) {
        return Some(TransitionBlocker::UnconfirmedScorecards);
    }
    None
}

fn owner_id(owner: ScoreOwner) -> Uuid {
    match owner {
        ScoreOwner::Player { id } | ScoreOwner::Team { id } => id,
    }
}

fn push_issue(
    issues: &mut Vec<CompletionIssue>,
    condition: bool,
    code: CompletionIssueCode,
    message: &'static str,
) {
    if condition {
        issues.push(CompletionIssue { code, message });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(id: u128, name: &str, holes: i64, confirmed: bool) -> OwnerProgressFact {
        OwnerProgressFact {
            owner: ScoreOwner::Player {
                id: Uuid::from_u128(id),
            },
            owner_name: name.to_owned(),
            holes_scored: holes,
            confirmed,
        }
    }

    #[test]
    fn validation_orders_owners_and_requires_complete_confirmed_cards() {
        let validation = validate(CompletionFacts {
            round_id: Uuid::from_u128(1),
            status: RoundStatus::Open,
            required_holes: 2,
            owners: vec![fact(3, "Zed", 2, true), fact(2, "Ada", 1, false)],
        });
        assert_eq!(validation.owners[0].owner_name, "Ada");
        assert!(!validation.ready_to_complete);
        assert_eq!(
            transition_blocker(&validation, TransitionAction::Complete),
            Some(TransitionBlocker::IncompleteScorecards)
        );
    }

    #[test]
    fn completion_and_lock_readiness_depend_on_source_state() {
        let facts = CompletionFacts {
            round_id: Uuid::from_u128(1),
            status: RoundStatus::Open,
            required_holes: 1,
            owners: vec![fact(2, "Ada", 1, true)],
        };
        let open = validate(facts.clone());
        assert!(open.ready_to_complete && !open.ready_to_lock);
        assert_eq!(transition_blocker(&open, TransitionAction::Complete), None);

        let completed = validate(CompletionFacts {
            status: RoundStatus::Completed,
            ..facts
        });
        assert!(!completed.ready_to_complete && completed.ready_to_lock);
        assert_eq!(transition_blocker(&completed, TransitionAction::Lock), None);
    }

    #[test]
    fn empty_owner_set_is_never_ready() {
        let validation = validate(CompletionFacts {
            round_id: Uuid::from_u128(1),
            status: RoundStatus::Open,
            required_holes: 18,
            owners: Vec::new(),
        });
        assert!(!validation.ready_to_complete);
        assert_eq!(
            transition_blocker(&validation, TransitionAction::Complete),
            Some(TransitionBlocker::NoRequiredOwners)
        );
    }
}
