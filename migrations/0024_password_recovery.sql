-- Authority history is independent of mutable membership rows. Changes away and
-- back cannot resurrect a capability; triggers do not lock another account.
CREATE TABLE password_recovery_authority_changes (
    version BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('user','membership','player','entrant')),
    subject_id UUID NOT NULL,
    context_id UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX password_recovery_authority_lookup
    ON password_recovery_authority_changes(kind,subject_id,version);

CREATE FUNCTION record_password_recovery_authority() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME='users' THEN
        IF NEW.role IS DISTINCT FROM OLD.role OR NEW.player_id IS DISTINCT FROM OLD.player_id THEN
            INSERT INTO password_recovery_authority_changes(kind,subject_id) VALUES('user',OLD.id);
        END IF;
    ELSIF TG_TABLE_NAME='players' THEN
        IF NEW.active IS DISTINCT FROM OLD.active THEN
            INSERT INTO password_recovery_authority_changes(kind,subject_id) VALUES('player',OLD.id);
        END IF;
    ELSIF TG_TABLE_NAME='tournament_memberships' THEN
        IF TG_OP='UPDATE' AND NEW.role=OLD.role AND NEW.user_id=OLD.user_id AND NEW.tournament_id=OLD.tournament_id THEN RETURN NEW; END IF;
        IF TG_OP<>'INSERT' THEN
            INSERT INTO password_recovery_authority_changes(kind,subject_id,context_id) VALUES('membership',OLD.user_id,OLD.tournament_id);
        END IF;
        IF TG_OP<>'DELETE' THEN
            INSERT INTO password_recovery_authority_changes(kind,subject_id,context_id) VALUES('membership',NEW.user_id,NEW.tournament_id);
        END IF;
    ELSE
        IF TG_OP='UPDATE' AND NEW.status=OLD.status AND NEW.player_id=OLD.player_id AND NEW.tournament_id=OLD.tournament_id THEN RETURN NEW; END IF;
        IF TG_OP<>'INSERT' THEN
            INSERT INTO password_recovery_authority_changes(kind,subject_id,context_id) VALUES('entrant',OLD.player_id,OLD.tournament_id);
        END IF;
        IF TG_OP<>'DELETE' THEN
            INSERT INTO password_recovery_authority_changes(kind,subject_id,context_id) VALUES('entrant',NEW.player_id,NEW.tournament_id);
        END IF;
    END IF;
    RETURN CASE WHEN TG_OP='DELETE' THEN OLD ELSE NEW END;
END;
$$;
CREATE TRIGGER recovery_user_authority AFTER UPDATE OF role,player_id ON users FOR EACH ROW EXECUTE FUNCTION record_password_recovery_authority();
CREATE TRIGGER recovery_player_authority AFTER UPDATE OF active ON players FOR EACH ROW EXECUTE FUNCTION record_password_recovery_authority();
CREATE TRIGGER recovery_membership_authority AFTER INSERT OR UPDATE OR DELETE ON tournament_memberships FOR EACH ROW EXECUTE FUNCTION record_password_recovery_authority();
CREATE TRIGGER recovery_entrant_authority AFTER INSERT OR UPDATE OR DELETE ON tournament_players FOR EACH ROW EXECUTE FUNCTION record_password_recovery_authority();

CREATE TABLE password_recovery_grants (
    id UUID PRIMARY KEY,
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash)=32),
    target_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    target_player_id UUID REFERENCES players(id) ON DELETE RESTRICT,
    credential_generation BIGINT NOT NULL CHECK (credential_generation>=0),
    authority_version BIGINT NOT NULL CHECK (authority_version>=0),
    issuer_kind TEXT NOT NULL CHECK (issuer_kind IN ('tournament_admin','operator')),
    issuer_user_id UUID REFERENCES users(id) ON DELETE RESTRICT,
    tournament_id UUID REFERENCES tournaments(id) ON DELETE RESTRICT,
    reason TEXT NOT NULL CHECK (length(btrim(reason)) BETWEEN 1 AND 500),
    issued_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    outcome TEXT CHECK (outcome IN ('replaced','revoked','redeemed')),
    ended_at TIMESTAMPTZ,
    ended_by UUID REFERENCES users(id) ON DELETE RESTRICT,
    end_reason TEXT,
    CHECK (expires_at > issued_at AND expires_at <= issued_at + interval '30 minutes'),
    CHECK ((outcome IS NULL)=(ended_at IS NULL)),
    CHECK ((issuer_kind='operator' AND issuer_user_id IS NULL AND tournament_id IS NULL)
        OR (issuer_kind='tournament_admin' AND issuer_user_id IS NOT NULL AND tournament_id IS NOT NULL
            AND target_player_id IS NOT NULL AND issuer_user_id<>target_user_id))
);
CREATE UNIQUE INDEX password_recovery_one_active ON password_recovery_grants(target_user_id) WHERE outcome IS NULL;
CREATE TABLE password_recovery_audits (
    id BIGSERIAL PRIMARY KEY,
    grant_id UUID NOT NULL REFERENCES password_recovery_grants(id) ON DELETE RESTRICT,
    outcome TEXT NOT NULL CHECK (outcome IN ('issued','replaced','revoked','redeemed')),
    actor_user_id UUID REFERENCES users(id) ON DELETE RESTRICT,
    database_actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    happened_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX password_recovery_audit_grant ON password_recovery_audits(grant_id,id);

-- Uses actual database authority, not a caller-controlled setting. Broad runtime
-- DML/EXECUTE grants cannot manufacture an operator grant. Operator uses the
-- migration/table owner connection; the application must never own these tables.
CREATE FUNCTION guard_password_recovery_grant() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' THEN RAISE EXCEPTION 'recovery grants are retained' USING ERRCODE='23514'; END IF;
    IF TG_OP='INSERT' THEN
        IF NEW.issuer_kind='operator' AND NOT EXISTS (
            SELECT 1 FROM pg_class c JOIN pg_roles r ON r.oid=c.relowner
            WHERE c.oid='password_recovery_grants'::regclass AND r.rolname=current_user
        ) AND NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname=current_user AND rolsuper) THEN
            RAISE EXCEPTION 'operator database authority required' USING ERRCODE='42501';
        END IF;
    ELSE
        IF (to_jsonb(NEW)-ARRAY['outcome','ended_at','ended_by','end_reason']) IS DISTINCT FROM
           (to_jsonb(OLD)-ARRAY['outcome','ended_at','ended_by','end_reason'])
           OR OLD.outcome IS NOT NULL OR NEW.outcome IS NULL THEN
            RAISE EXCEPTION 'recovery grant identity and terminal outcome are immutable' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER password_recovery_grant_guard BEFORE INSERT OR UPDATE OR DELETE ON password_recovery_grants FOR EACH ROW EXECUTE FUNCTION guard_password_recovery_grant();
CREATE FUNCTION audit_password_recovery_grant() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO password_recovery_audits(grant_id,outcome,actor_user_id,database_actor,reason)
    VALUES(NEW.id,CASE WHEN TG_OP='INSERT' THEN 'issued' ELSE NEW.outcome END,
        CASE WHEN TG_OP='INSERT' THEN NEW.issuer_user_id ELSE NEW.ended_by END,current_user,
        CASE WHEN TG_OP='INSERT' THEN NEW.reason ELSE COALESCE(NEW.end_reason,NEW.outcome) END);
    RETURN NEW;
END;
$$;
CREATE TRIGGER password_recovery_grant_audit AFTER INSERT OR UPDATE ON password_recovery_grants FOR EACH ROW EXECUTE FUNCTION audit_password_recovery_grant();
CREATE FUNCTION guard_password_recovery_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP<>'INSERT' OR pg_trigger_depth()<2 THEN
        RAISE EXCEPTION 'recovery history is append-only and workflow-owned' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER password_recovery_audit_guard BEFORE INSERT OR UPDATE OR DELETE ON password_recovery_audits FOR EACH ROW EXECUTE FUNCTION guard_password_recovery_history();
CREATE TRIGGER password_recovery_authority_guard BEFORE INSERT OR UPDATE OR DELETE ON password_recovery_authority_changes FOR EACH ROW EXECUTE FUNCTION guard_password_recovery_history();
