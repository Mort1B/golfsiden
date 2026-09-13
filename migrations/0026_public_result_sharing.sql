BEGIN;

-- Capability authority belongs to the tournament. Issuer/revoker UUIDs are audit
-- snapshots without user FKs so account removal does not remove shared results.
CREATE TABLE tournament_result_shares (
    id UUID PRIMARY KEY,
    tournament_id UUID NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    UNIQUE(id,tournament_id),
    token_hash BYTEA NOT NULL UNIQUE CHECK(octet_length(token_hash)=32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    created_by UUID NOT NULL,
    revoked_at TIMESTAMPTZ,
    revoked_by UUID,
    CHECK(expires_at = created_at + interval '30 days'),
    CHECK((revoked_at IS NULL) = (revoked_by IS NULL)),
    CHECK(revoked_at IS NULL OR revoked_at >= created_at)
);
CREATE UNIQUE INDEX tournament_result_shares_one_live ON tournament_result_shares(tournament_id) WHERE revoked_at IS NULL;
CREATE INDEX tournament_result_shares_latest ON tournament_result_shares(tournament_id,created_at DESC,id DESC);
CREATE TABLE tournament_result_share_audits (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tournament_id UUID NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    grant_id UUID NOT NULL,
    FOREIGN KEY(grant_id,tournament_id) REFERENCES tournament_result_shares(id,tournament_id) ON DELETE CASCADE,
    actor_user_id UUID NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('issued','replaced','revoked')),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE FUNCTION guard_tournament_result_share() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE actor UUID;
BEGIN
    IF TG_OP='DELETE' THEN
        IF EXISTS(SELECT 1 FROM tournaments WHERE id=OLD.tournament_id) THEN
            RAISE EXCEPTION 'result link history is retained' USING ERRCODE='23514',CONSTRAINT='result_share_delete_forbidden';
        END IF;
        RETURN OLD;
    END IF;
    IF TG_OP='INSERT' AND NEW.revoked_at IS NOT NULL THEN
        RAISE EXCEPTION 'new result links must be active' USING ERRCODE='23514',CONSTRAINT='result_share_new_active';
    END IF;
    IF TG_OP='UPDATE'  AND
      (NEW.id,NEW.tournament_id,NEW.token_hash,NEW.created_at,NEW.expires_at,NEW.created_by)
      IS DISTINCT FROM
      (OLD.id,OLD.tournament_id,OLD.token_hash,OLD.created_at,OLD.expires_at,OLD.created_by) THEN
        RAISE EXCEPTION 'result link identity is immutable' USING ERRCODE='23514',CONSTRAINT='result_share_immutable';
    END IF;
    IF TG_OP='UPDATE' AND OLD.revoked_at IS NOT NULL THEN
        RAISE EXCEPTION 'revoked result links cannot change' USING ERRCODE='23514',CONSTRAINT='result_share_terminal';
    END IF;
    SELECT s.user_id INTO actor FROM user_sessions s
      JOIN users u ON u.id=s.user_id AND u.credential_generation=s.credential_generation
      JOIN tournament_memberships m ON m.user_id=s.user_id
      WHERE s.id=NULLIF(current_setting('app.result_share_session_id',true),'')::UUID
      AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp()
      AND m.tournament_id=NEW.tournament_id AND m.role='admin';
    IF actor IS NULL OR (TG_OP='INSERT' AND NEW.created_by IS DISTINCT FROM actor)
       OR (TG_OP='UPDATE' AND (NEW.revoked_at IS NULL OR NEW.revoked_by IS DISTINCT FROM actor)) THEN
        RAISE EXCEPTION 'result links require current tournament administrator' USING ERRCODE='23514',CONSTRAINT='result_share_admin_required';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER tournament_result_shares_guard BEFORE INSERT OR UPDATE OR DELETE ON tournament_result_shares
FOR EACH ROW EXECUTE FUNCTION guard_tournament_result_share();

CREATE FUNCTION audit_tournament_result_share() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO tournament_result_share_audits(tournament_id,grant_id,actor_user_id,action)
      VALUES(NEW.tournament_id,NEW.id,CASE WHEN TG_OP='INSERT' THEN NEW.created_by ELSE NEW.revoked_by END,
      CASE WHEN TG_OP='INSERT' THEN 'issued'
           WHEN current_setting('app.result_share_replacing',true)='true' THEN 'replaced' ELSE 'revoked' END);
    RETURN NEW;
END;
$$;
CREATE TRIGGER tournament_result_shares_audit AFTER INSERT OR UPDATE ON tournament_result_shares
FOR EACH ROW EXECUTE FUNCTION audit_tournament_result_share();
CREATE FUNCTION guard_result_share_audit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='INSERT' AND pg_trigger_depth()>1 THEN RETURN NEW; END IF;
    IF TG_OP='DELETE' AND NOT EXISTS(SELECT 1 FROM tournaments WHERE id=OLD.tournament_id) THEN RETURN OLD; END IF;
    RAISE EXCEPTION 'result link audits are derived immutable history' USING ERRCODE='23514',CONSTRAINT='result_share_audit_immutable';
END;
$$;
CREATE TRIGGER tournament_result_share_audits_guard BEFORE INSERT OR UPDATE OR DELETE ON tournament_result_share_audits
FOR EACH ROW EXECUTE FUNCTION guard_result_share_audit();
COMMIT;
