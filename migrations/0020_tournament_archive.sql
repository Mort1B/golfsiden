BEGIN;

-- Valid pre-workflow closed history is retained without inventing audit actors.
CREATE TABLE tournament_archives (
    tournament_id UUID PRIMARY KEY REFERENCES tournaments(id) ON DELETE RESTRICT,
    archived_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    archived_at TIMESTAMPTZ NOT NULL
);

-- Completion remains unchanged except that archive now has its own guard.
CREATE OR REPLACE FUNCTION guard_tournament_completion() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    completion_session UUID;
    actor UUID;
BEGIN
    IF NEW.status IS NOT DISTINCT FROM OLD.status THEN RETURN NEW; END IF;
    IF OLD.status <> 'active' OR NEW.status <> 'completed' THEN RETURN NEW; END IF;
    IF current_setting('app.tournament_completion_id', true) IS DISTINCT FROM OLD.id::TEXT THEN
        RAISE EXCEPTION 'completion requires the tournament workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_completion_context_required';
    END IF;
    BEGIN
        completion_session = current_setting('app.tournament_completion_session_id', true)::UUID;
    EXCEPTION WHEN invalid_text_representation THEN
        RAISE EXCEPTION 'completion requires an active exact tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_completion_admin_required';
    END;
    SELECT s.user_id INTO actor FROM user_sessions s
    JOIN tournament_memberships m ON m.user_id = s.user_id
    WHERE s.id = completion_session AND s.revoked_at IS NULL
      AND s.expires_at > clock_timestamp() AND m.tournament_id = OLD.id AND m.role = 'admin'
    FOR SHARE OF s, m;
    IF actor IS NULL OR NOT EXISTS (
        SELECT 1 FROM user_sessions WHERE id = completion_session
        AND revoked_at IS NULL AND expires_at > clock_timestamp()
    ) THEN
        RAISE EXCEPTION 'completion requires an active exact tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_completion_admin_required';
    END IF;
    IF (SELECT count(*) FROM rounds WHERE tournament_id = OLD.id) <> OLD.number_of_rounds
       OR EXISTS (SELECT 1 FROM rounds WHERE tournament_id = OLD.id
                  AND (status <> 'locked' OR round_number NOT BETWEEN 1 AND OLD.number_of_rounds)) THEN
        RAISE EXCEPTION 'completion requires every configured round to be locked'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_completion_not_ready';
    END IF;
    INSERT INTO tournament_completions (tournament_id, completed_by_user_id, completed_at)
    VALUES (OLD.id, actor, clock_timestamp());
    RETURN NEW;
END;
$$;

CREATE FUNCTION guard_tournament_archive() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    archive_session UUID;
    actor UUID;
BEGIN
    IF NEW.status IS NOT DISTINCT FROM OLD.status THEN RETURN NEW; END IF;
    IF OLD.status <> 'completed' OR NEW.status <> 'archived' THEN RETURN NEW; END IF;
    IF current_setting('app.tournament_archive_id', true) IS DISTINCT FROM OLD.id::TEXT THEN
        RAISE EXCEPTION 'archive requires the tournament workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_archive_context_required';
    END IF;
    BEGIN
        archive_session = current_setting('app.tournament_archive_session_id', true)::UUID;
    EXCEPTION WHEN invalid_text_representation THEN
        RAISE EXCEPTION 'archive requires an active exact tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_archive_admin_required';
    END;
    SELECT s.user_id INTO actor FROM user_sessions s
    JOIN tournament_memberships m ON m.user_id = s.user_id
    WHERE s.id = archive_session AND s.revoked_at IS NULL
      AND s.expires_at > clock_timestamp() AND m.tournament_id = OLD.id AND m.role = 'admin'
    FOR SHARE OF s, m;
    IF actor IS NULL OR NOT EXISTS (
        SELECT 1 FROM user_sessions WHERE id = archive_session
        AND revoked_at IS NULL AND expires_at > clock_timestamp()
    ) THEN
        RAISE EXCEPTION 'archive requires an active exact tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_archive_admin_required';
    END IF;
    INSERT INTO tournament_archives (tournament_id, archived_by_user_id, archived_at)
    VALUES (OLD.id, actor, clock_timestamp());
    RETURN NEW;
END;
$$;

CREATE TRIGGER tournaments_guard_archive
BEFORE UPDATE OF status ON tournaments
FOR EACH ROW EXECUTE FUNCTION guard_tournament_archive();

CREATE FUNCTION protect_tournament_archive_record() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- Only the guarded parent transition inserts a record. Runtime roles cannot
    -- create their own triggers; schema owners remain trusted migration authority.
    IF TG_OP = 'INSERT' AND pg_trigger_depth() = 2
       AND current_setting('app.tournament_archive_id', true) = NEW.tournament_id::TEXT THEN
        RETURN NEW;
    END IF;
    -- Preserve archive time/identity when the user's FK deliberately nulls
    -- its actor reference. Ordinary attempts to erase a still-existing actor fail.
    IF TG_OP = 'UPDATE' AND NEW.tournament_id = OLD.tournament_id
       AND NEW.archived_at = OLD.archived_at AND NEW.archived_by_user_id IS NULL
       AND OLD.archived_by_user_id IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM users WHERE id = OLD.archived_by_user_id) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'archive records are workflow-managed and append-only'
        USING ERRCODE = '23514', CONSTRAINT = 'tournament_archive_record_immutable';
END;
$$;
CREATE TRIGGER tournament_archives_protect_record
BEFORE INSERT OR UPDATE OR DELETE ON tournament_archives
FOR EACH ROW EXECUTE FUNCTION protect_tournament_archive_record();

COMMIT;
