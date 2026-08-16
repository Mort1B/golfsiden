ALTER TABLE users
    ADD CONSTRAINT users_id_player_id_unique UNIQUE (id, player_id);

ALTER TABLE tournament_invitations
    ADD COLUMN series_id UUID,
    ADD COLUMN predecessor_id UUID,
    ADD COLUMN revoked_by_user_id UUID,
    ADD COLUMN revocation_actor_known BOOLEAN NOT NULL DEFAULT TRUE;

-- Invitations issued before reusable links existed are roots of their own series.
UPDATE tournament_invitations
SET series_id = id,
    revocation_actor_known = revoked_at IS NULL;

ALTER TABLE tournament_invitations
    ALTER COLUMN series_id SET NOT NULL,
    DROP CONSTRAINT tournament_invitations_tournament_id_created_by_user_id_fkey,
    ADD CONSTRAINT tournament_invitations_tournament_id_id_unique
        UNIQUE (tournament_id, id),
    ADD CONSTRAINT tournament_invitations_tournament_id_id_series_unique
        UNIQUE (tournament_id, id, series_id),
    ADD CONSTRAINT tournament_invitations_creator_membership_fkey
        FOREIGN KEY (tournament_id, created_by_user_id)
        REFERENCES tournament_memberships(tournament_id, user_id)
        DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT tournament_invitations_series_fkey
        FOREIGN KEY (tournament_id, series_id)
        REFERENCES tournament_invitations(tournament_id, id)
        DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT tournament_invitations_predecessor_fkey
        FOREIGN KEY (tournament_id, predecessor_id)
        REFERENCES tournament_invitations(tournament_id, id)
        DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT tournament_invitations_revoker_membership_fkey
        FOREIGN KEY (tournament_id, revoked_by_user_id)
        REFERENCES tournament_memberships(tournament_id, user_id)
        DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT tournament_invitations_revocation_provenance
        CHECK (
            (revoked_at IS NULL
             AND revoked_by_user_id IS NULL
             AND revocation_actor_known)
            OR
            (revoked_at IS NOT NULL
             AND (
                 (revocation_actor_known AND revoked_by_user_id IS NOT NULL)
                 OR
                 (NOT revocation_actor_known AND revoked_by_user_id IS NULL)
             ))
        );

CREATE UNIQUE INDEX tournament_invitations_one_successor_idx
    ON tournament_invitations (tournament_id, predecessor_id)
    WHERE predecessor_id IS NOT NULL;

CREATE INDEX tournament_invitations_series_created_idx
    ON tournament_invitations (tournament_id, series_id, created_at, id);

CREATE FUNCTION enforce_invitation_series_policy() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    predecessor tournament_invitations%ROWTYPE;
BEGIN
    IF NEW.predecessor_id IS NULL THEN
        IF NEW.series_id <> NEW.id THEN
            RAISE EXCEPTION 'a fresh invitation must be its own series root'
                USING ERRCODE = '23514',
                      CONSTRAINT = 'invitation_series_root_required';
        END IF;
        RETURN NEW;
    END IF;

    SELECT * INTO predecessor
    FROM tournament_invitations
    WHERE tournament_id = NEW.tournament_id
      AND id = NEW.predecessor_id;

    IF NOT FOUND
       OR predecessor.revoked_at IS NULL
       OR NEW.series_id <> predecessor.series_id
       OR NEW.expires_at <> predecessor.expires_at
       OR NEW.max_uses IS DISTINCT FROM predecessor.max_uses THEN
        RAISE EXCEPTION 'an invitation successor must inherit its series policy'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_successor_policy_mismatch';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tournament_invitations_enforce_series_policy
BEFORE INSERT ON tournament_invitations
FOR EACH ROW EXECUTE FUNCTION enforce_invitation_series_policy();

CREATE FUNCTION protect_invitation_identity() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.tournament_id IS DISTINCT FROM OLD.tournament_id
       OR NEW.token_hash IS DISTINCT FROM OLD.token_hash
       OR NEW.created_by_user_id IS DISTINCT FROM OLD.created_by_user_id
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
       OR NEW.max_uses IS DISTINCT FROM OLD.max_uses
       OR NEW.series_id IS DISTINCT FROM OLD.series_id
       OR NEW.predecessor_id IS DISTINCT FROM OLD.predecessor_id
       OR NEW.revocation_actor_known IS DISTINCT FROM OLD.revocation_actor_known THEN
        RAISE EXCEPTION 'invitation identity and policy are immutable'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_identity_immutable';
    END IF;
    IF OLD.revoked_at IS NOT NULL
       AND (NEW.revoked_at IS DISTINCT FROM OLD.revoked_at
            OR NEW.revoked_by_user_id IS DISTINCT FROM OLD.revoked_by_user_id) THEN
        RAISE EXCEPTION 'invitation revocation is immutable'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_revocation_immutable';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tournament_invitations_protect_identity
BEFORE UPDATE ON tournament_invitations
FOR EACH ROW EXECUTE FUNCTION protect_invitation_identity();

CREATE TYPE invitation_redemption_mode AS ENUM ('registration', 'acceptance');

CREATE TABLE invitation_redemptions (
    id UUID PRIMARY KEY,
    invitation_id UUID NOT NULL,
    series_id UUID NOT NULL,
    tournament_id UUID NOT NULL
        REFERENCES tournaments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    player_id UUID NOT NULL,
    mode invitation_redemption_mode NOT NULL,
    redeemed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (tournament_id, invitation_id, series_id)
        REFERENCES tournament_invitations(tournament_id, id, series_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (user_id, player_id)
        REFERENCES users(id, player_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (tournament_id, user_id)
        REFERENCES tournament_memberships(tournament_id, user_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (tournament_id, player_id)
        REFERENCES tournament_players(tournament_id, player_id)
        DEFERRABLE INITIALLY DEFERRED,
    UNIQUE (tournament_id, user_id),
    UNIQUE (tournament_id, player_id)
);

-- The capacity check counts committed facts by stable series root while holding
-- the root invitation lock.
CREATE INDEX invitation_redemptions_series_capacity_idx
    ON invitation_redemptions (tournament_id, series_id, redeemed_at, id);

CREATE FUNCTION enforce_invitation_redemption_insert() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    target_invitation tournament_invitations%ROWTYPE;
    target_tournament_status tournament_status;
    linked_player_id UUID;
    series_redemptions BIGINT;
    checked_at TIMESTAMPTZ;
BEGIN
    -- Direct writes overlap authenticated acceptance at the identity rows, so
    -- acquire every shared lock in its application order before invitations.
    SELECT u.player_id INTO linked_player_id
    FROM users u
    WHERE u.id = NEW.user_id
    FOR UPDATE;
    IF NOT FOUND OR linked_player_id IS DISTINCT FROM NEW.player_id THEN
        RAISE EXCEPTION 'redemption user-player linkage is invalid'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_user_player_invalid';
    END IF;

    PERFORM p.id
    FROM players p
    WHERE p.id = NEW.player_id
    FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'redemption player is missing'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_player_missing';
    END IF;

    PERFORM tm.user_id
    FROM tournament_memberships tm
    WHERE tm.tournament_id = NEW.tournament_id
      AND tm.user_id = NEW.user_id
    FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'redemption tournament membership is missing'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_membership_missing';
    END IF;

    SELECT i.* INTO target_invitation
    FROM tournament_invitations i
    WHERE i.id = NEW.invitation_id
    FOR UPDATE;

    IF NOT FOUND
       OR NEW.tournament_id <> target_invitation.tournament_id
       OR NEW.series_id <> target_invitation.series_id THEN
        RAISE EXCEPTION 'redemption invitation linkage is invalid'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_target_invalid';
    END IF;

    IF target_invitation.series_id <> target_invitation.id THEN
        PERFORM id
        FROM tournament_invitations
        WHERE id = target_invitation.series_id
          AND tournament_id = target_invitation.tournament_id
        FOR UPDATE;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'redemption invitation series is invalid'
                USING ERRCODE = '23514',
                      CONSTRAINT = 'invitation_redemption_series_invalid';
        END IF;
    END IF;

    PERFORM tp.player_id
    FROM tournament_players tp
    WHERE tp.tournament_id = target_invitation.tournament_id
      AND tp.player_id = NEW.player_id
    FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'redemption tournament entrant is missing'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_entrant_missing';
    END IF;

    SELECT status INTO target_tournament_status
    FROM tournaments
    WHERE id = target_invitation.tournament_id;
    checked_at = clock_timestamp();
    IF target_invitation.revoked_at IS NOT NULL THEN
        RAISE EXCEPTION 'revoked invitations cannot be redeemed'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_revoked';
    END IF;
    IF target_invitation.expires_at <= checked_at THEN
        RAISE EXCEPTION 'expired invitations cannot be redeemed'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_expired';
    END IF;
    IF target_tournament_status NOT IN ('draft', 'active') THEN
        RAISE EXCEPTION 'the tournament is not accepting invitation redemptions'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_tournament_closed';
    END IF;

    SELECT count(*) INTO series_redemptions
    FROM invitation_redemptions
    WHERE tournament_id = target_invitation.tournament_id
      AND series_id = target_invitation.series_id;
    IF target_invitation.max_uses IS NOT NULL
       AND series_redemptions >= target_invitation.max_uses THEN
        RAISE EXCEPTION 'invitation series capacity is exhausted'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_capacity_exhausted';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER invitation_redemptions_enforce_insert
BEFORE INSERT ON invitation_redemptions
FOR EACH ROW EXECUTE FUNCTION enforce_invitation_redemption_insert();

CREATE FUNCTION protect_invitation_redemption() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'invitation redemption facts are immutable'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_immutable';
    END IF;
    -- Referential cascades execute this trigger recursively. Direct deletion is
    -- rejected, but a whole-tournament cascade may remove the preserved facts.
    IF TG_OP = 'DELETE' AND pg_trigger_depth() <= 1 THEN
        RAISE EXCEPTION 'invitation redemption facts are append-only'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'invitation_redemption_append_only';
    END IF;
    RETURN OLD;
END;
$$;

CREATE TRIGGER invitation_redemptions_protect_facts
BEFORE UPDATE OR DELETE ON invitation_redemptions
FOR EACH ROW EXECUTE FUNCTION protect_invitation_redemption();
