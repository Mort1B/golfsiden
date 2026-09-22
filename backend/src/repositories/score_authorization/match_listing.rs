//! One listing's authority, valid only while its caller holds the same transaction.
use super::*;
use std::collections::HashSet;

pub(crate) struct MatchListingContext {
    principal: SessionPrincipal,
    role: TournamentRole,
}
impl MatchListingContext {
    // The tournament is resolved from the stored round in the caller's transaction.
    pub(crate) async fn load(
        tx: &mut Transaction<'_, Postgres>,
        session: Uuid,
        tournament: Uuid,
    ) -> Result<Self, ScoreAuthorizationError> {
        let principal = auth::lock_active_session(tx, session)
            .await?
            .ok_or(ScoreAuthorizationError::Unauthenticated)?;
        let role = membership_role(tx, tournament, principal.user_id)
            .await?
            .ok_or(ScoreAuthorizationError::Forbidden)?;
        Ok(Self { principal, role })
    }
    pub(crate) fn role(&self) -> TournamentRole {
        self.role
    }
    pub(crate) async fn players(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        round: Uuid,
    ) -> Result<HashSet<Uuid>, ScoreAuthorizationError> {
        Ok(resolve_owners(
            tx,
            &self.principal,
            Some(self.role),
            round,
            ScoringFormat::SinglesMatchPlay,
        )
        .await?
        .into_iter()
        .filter_map(|owner| owner.player_id())
        .collect())
    }
}
