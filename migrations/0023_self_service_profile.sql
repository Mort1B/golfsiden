-- Existing sessions remain valid until the next password change. Generations
-- invalidate sessions without acquiring other session rows (and lock cycles).
ALTER TABLE users ADD COLUMN credential_generation BIGINT NOT NULL DEFAULT 0 CHECK (credential_generation >= 0);
ALTER TABLE users ADD COLUMN profile_version BIGINT NOT NULL DEFAULT 0 CHECK (profile_version >= 0);
ALTER TABLE user_sessions ADD COLUMN credential_generation BIGINT NOT NULL DEFAULT 0 CHECK (credential_generation >= 0);

CREATE FUNCTION advance_account_version() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.profile_version = OLD.profile_version + 1;
    NEW.credential_generation = OLD.credential_generation
        + CASE WHEN NEW.password_hash IS DISTINCT FROM OLD.password_hash THEN 1 ELSE 0 END;
    NEW.updated_at = clock_timestamp();
    RETURN NEW;
END;
$$;
CREATE TRIGGER users_advance_account_version BEFORE UPDATE ON users
FOR EACH ROW EXECUTE FUNCTION advance_account_version();

-- Supplement the existing lifecycle/visibility guards with the same credential
-- boundary as HTTP authorization. Keep their membership/readiness/audit rules.
CREATE FUNCTION guard_tournament_session_generation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    session_setting TEXT;
    valid_session UUID;
    failure_constraint TEXT;
BEGIN
    IF OLD.status = 'active' AND NEW.status = 'completed' THEN
        session_setting = 'app.tournament_completion_session_id';
        failure_constraint = 'tournament_completion_admin_required';
    ELSIF OLD.status = 'completed' AND NEW.status = 'archived' THEN
        session_setting = 'app.tournament_archive_session_id';
        failure_constraint = 'tournament_archive_admin_required';
    ELSIF NEW.final_round_back_nine_hidden IS DISTINCT FROM OLD.final_round_back_nine_hidden THEN
        session_setting = 'app.final_round_visibility_session_id';
        failure_constraint = 'final_round_visibility_admin_required';
    ELSE
        RETURN NEW;
    END IF;
    -- Missing/invalid context remains the responsibility of existing guards.
    IF NULLIF(current_setting(session_setting, true), '') IS NULL THEN RETURN NEW; END IF;
    SELECT s.id INTO valid_session FROM user_sessions s JOIN users u ON u.id=s.user_id
    WHERE s.id::TEXT = current_setting(session_setting, true)
      AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
      AND s.credential_generation = u.credential_generation
    FOR SHARE OF s, u;
    IF valid_session IS NULL OR NOT EXISTS (
        SELECT 1 FROM user_sessions WHERE id=valid_session AND expires_at > clock_timestamp()
    ) THEN
        RAISE EXCEPTION 'workflow requires a current credential generation'
          USING ERRCODE='23514', CONSTRAINT=failure_constraint;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER tournaments_guard_session_generation
BEFORE UPDATE OF status, final_round_back_nine_hidden ON tournaments
FOR EACH ROW EXECUTE FUNCTION guard_tournament_session_generation();
