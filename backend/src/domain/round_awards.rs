//! Personal competition awards never change native hole/card facts.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "round_award_type", rename_all = "snake_case")]
pub enum AwardType {
    ClosestToPin,
    LongestDrive,
}
#[derive(Debug, Clone)]
pub struct AwardFact {
    pub player_id: Uuid,
    pub award_type: AwardType,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NetAwards {
    Stroke {
        awards: Vec<AwardType>,
        adjustment: i32,
        base_net_total: i32,
        adjusted_net_total: i32,
    },
    Stableford {
        awards: Vec<AwardType>,
        adjustment: i32,
        base_net_points: i32,
        competition_net_points: i32,
        base_net_equivalent: i32,
        adjusted_net_equivalent: i32,
    },
}
impl NetAwards {
    pub fn adjustment(&self) -> i32 {
        match self {
            Self::Stroke { adjustment, .. } | Self::Stableford { adjustment, .. } => *adjustment,
        }
    }
}
pub fn summary(
    facts: &[AwardFact],
    player: Uuid,
    net: i32,
    stableford: Option<&crate::domain::stableford::card::Values>,
) -> Option<NetAwards> {
    let mut awards: Vec<_> = facts
        .iter()
        .filter(|f| f.player_id == player)
        .map(|f| f.award_type)
        .collect();
    awards.sort();
    awards.dedup();
    let adjustment = -(awards.len() as i32);
    if awards.is_empty() {
        return None;
    }
    Some(match stableford {
        Some(v) => NetAwards::Stableford {
            awards,
            adjustment,
            base_net_points: v.net_points,
            competition_net_points: v.net_points - adjustment,
            base_net_equivalent: v.net_equivalent,
            adjusted_net_equivalent: v.net_equivalent + adjustment,
        },
        None => NetAwards::Stroke {
            awards,
            adjustment,
            base_net_total: net,
            adjusted_net_total: net + adjustment,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_either_both_and_duplicate_awards_have_bounded_personal_adjustments() {
        let player = Uuid::from_u128(1);
        assert!(summary(&[], player, 72, None).is_none());
        let closest = AwardFact {
            player_id: player,
            award_type: AwardType::ClosestToPin,
        };
        let drive = AwardFact {
            player_id: player,
            award_type: AwardType::LongestDrive,
        };
        for award in [&closest, &drive] {
            assert_eq!(
                summary(std::slice::from_ref(award), player, 72, None)
                    .unwrap()
                    .adjustment(),
                -1
            );
        }
        let result = summary(&[closest.clone(), drive.clone(), closest], player, 72, None).unwrap();
        assert!(matches!(
            result,
            NetAwards::Stroke {
                base_net_total: 72,
                adjusted_net_total: 70,
                ..
            }
        ));
        assert!(summary(&[drive], Uuid::from_u128(2), 72, None).is_none());
    }
}
