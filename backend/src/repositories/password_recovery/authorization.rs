use super::*;

pub(super) async fn lock_admin_session(
    tx: &mut Transaction<'_, Postgres>,
    request: &AdminRequest,
) -> Result<(), RecoveryError> {
    // Session first, before either account. Do not use the joined session/user
    // helper: profile writes already take this session then its single account.
    let found: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM user_sessions WHERE id=$1 AND user_id=$2 FOR SHARE")
            .bind(request.session_id)
            .bind(request.user_id)
            .fetch_optional(&mut **tx)
            .await?;
    if found.is_none() {
        return Err(RecoveryError::Unauthenticated);
    }
    Ok(())
}
pub(super) async fn require_session(
    tx: &mut Transaction<'_, Postgres>,
    request: &AdminRequest,
) -> Result<(), RecoveryError> {
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_sessions s JOIN users u ON u.id=s.user_id WHERE s.id=$1 AND s.user_id=$2 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND s.credential_generation=u.credential_generation)")
        .bind(request.session_id).bind(request.user_id).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(RecoveryError::Unauthenticated);
    }
    Ok(())
}
pub(super) async fn eligible(
    tx: &mut Transaction<'_, Postgres>,
    issuer: Uuid,
    target: &Account,
    tournament: Uuid,
    player: Uuid,
) -> Result<(), RecoveryError> {
    if issuer == target.id
        || target.is_admin
        || target.player_id != Some(player)
        || target.password_hash.is_none()
    {
        return Err(RecoveryError::Forbidden);
    }
    let active: Option<bool> =
        sqlx::query_scalar("SELECT active FROM players WHERE id=$1 FOR SHARE")
            .bind(player)
            .fetch_optional(&mut **tx)
            .await?;
    // Existing membership updates are frozen; new memberships need FK KEY SHARE
    // on the target account, blocked by its FOR UPDATE serialization lock.
    let memberships: Vec<(Uuid,Uuid,String)> = sqlx::query_as("SELECT user_id,tournament_id,role::text FROM tournament_memberships WHERE user_id=$1 OR (user_id=$2 AND tournament_id=$3) ORDER BY user_id,tournament_id FOR SHARE")
        .bind(target.id).bind(issuer).bind(tournament).fetch_all(&mut **tx).await?;
    if !memberships
        .iter()
        .any(|(u, t, r)| *u == issuer && *t == tournament && r == "admin")
        || !memberships
            .iter()
            .any(|(u, t, _)| *u == target.id && *t == tournament)
        || memberships
            .iter()
            .any(|(u, _, r)| *u == target.id && r == "admin")
    {
        return Err(RecoveryError::Forbidden);
    }
    let entrant: Option<bool> = sqlx::query_scalar("SELECT status='active' FROM tournament_players WHERE tournament_id=$1 AND player_id=$2 FOR SHARE")
        .bind(tournament).bind(player).fetch_optional(&mut **tx).await?;
    if active != Some(true) || entrant != Some(true) {
        return Err(RecoveryError::Forbidden);
    }
    Ok(())
}
pub(super) async fn admin_target(
    tx: &mut Transaction<'_, Postgres>,
    request: &AdminRequest,
) -> Result<Account, RecoveryError> {
    lock_admin_session(tx, request).await?;
    let target: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE player_id=$1")
        .bind(request.player_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(RecoveryError::Forbidden)?;
    let mut locked = accounts(tx, &[target, request.user_id]).await?;
    let issuer = locked
        .iter()
        .find(|a| a.id == request.user_id)
        .ok_or(RecoveryError::Unauthenticated)?;
    if issuer.password_hash.as_ref() != Some(&request.verified_password.0)
        || issuer.credential_generation != request.verified_password.1
    {
        return Err(RecoveryError::IncorrectPassword);
    }
    require_session(tx, request).await?;
    let position = locked
        .iter()
        .position(|a| a.id == target)
        .ok_or(RecoveryError::Forbidden)?;
    let target = locked.remove(position);
    eligible(
        tx,
        request.user_id,
        &target,
        request.tournament_id,
        request.player_id,
    )
    .await?;
    require_session(tx, request).await?;
    Ok(target)
}
pub(super) async fn unchanged(
    tx: &mut Transaction<'_, Postgres>,
    grant: &Grant,
) -> Result<bool, RecoveryError> {
    Ok(!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM password_recovery_authority_changes WHERE version>$1 AND ((kind='user' AND subject_id=$2) OR (kind='membership' AND (subject_id=$2 OR (subject_id=$3 AND context_id=$4))) OR (kind='player' AND subject_id=$5) OR (kind='entrant' AND subject_id=$5 AND context_id=$4)))")
        .bind(grant.authority_version).bind(grant.target_user_id).bind(grant.issuer_user_id).bind(grant.tournament_id).bind(grant.target_player_id).fetch_one(&mut **tx).await?)
}
