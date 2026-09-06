BEGIN;

-- Do not silently relabel or rewrite closed historical results.
DO $$ BEGIN
    IF EXISTS (
        SELECT 1 FROM tournaments t
        WHERE t.status IN ('completed', 'archived') AND (
            (SELECT count(*) FROM rounds r WHERE r.tournament_id = t.id) <> t.number_of_rounds
            OR EXISTS (SELECT 1 FROM rounds r WHERE r.tournament_id = t.id
                       AND (r.status <> 'locked' OR r.round_number NOT BETWEEN 1 AND t.number_of_rounds))
        )
    ) THEN
        RAISE EXCEPTION 'closed tournament history must have every configured round locked before schema 19'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_completion_legacy_not_ready';
    END IF;
END $$;

CREATE TABLE tournament_completions (
    tournament_id UUID PRIMARY KEY REFERENCES tournaments(id) ON DELETE RESTRICT,
    completed_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    completed_at TIMESTAMPTZ NOT NULL
);

CREATE FUNCTION guard_tournament_completion() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    completion_session UUID;
    actor UUID;
BEGIN
    IF NEW.status IS NOT DISTINCT FROM OLD.status THEN RETURN NEW; END IF;
    IF OLD.status = 'completed' AND NEW.status = 'archived' THEN
        RAISE EXCEPTION 'archiving requires a separately implemented workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_archive_unavailable';
    END IF;
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

CREATE TRIGGER tournaments_guard_completion
BEFORE UPDATE OF status ON tournaments
FOR EACH ROW EXECUTE FUNCTION guard_tournament_completion();

CREATE FUNCTION protect_tournament_completion_record() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- Only the guarded parent transition inserts a record. Runtime roles cannot
    -- create their own triggers; schema owners remain trusted migration authority.
    IF TG_OP = 'INSERT' AND pg_trigger_depth() = 2
       AND current_setting('app.tournament_completion_id', true) = NEW.tournament_id::TEXT THEN
        RETURN NEW;
    END IF;
    -- Preserve completion time/identity when the user's FK deliberately nulls
    -- its actor reference. Ordinary attempts to erase a still-existing actor fail.
    IF TG_OP = 'UPDATE' AND NEW.tournament_id = OLD.tournament_id
       AND NEW.completed_at = OLD.completed_at AND NEW.completed_by_user_id IS NULL
       AND OLD.completed_by_user_id IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM users WHERE id = OLD.completed_by_user_id) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'completion records are workflow-managed and append-only'
        USING ERRCODE = '23514', CONSTRAINT = 'tournament_completion_record_immutable';
END;
$$;
CREATE TRIGGER tournament_completions_protect_record
BEFORE INSERT OR UPDATE OR DELETE ON tournament_completions
FOR EACH ROW EXECUTE FUNCTION protect_tournament_completion_record();

-- Hold the parent lock until commit: a join/issue which wins the lock may finish;
-- one which loses to completion sees the closed state and rolls back.
CREATE FUNCTION require_joinable_tournament() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE parent_status tournament_status;
BEGIN
    SELECT status INTO parent_status FROM tournaments WHERE id = NEW.tournament_id FOR SHARE;
    IF parent_status IN ('completed', 'archived') THEN
        RAISE EXCEPTION 'the tournament is closed to new invitations and members'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_closed_to_joining';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER invitations_require_joinable_tournament BEFORE INSERT ON tournament_invitations
FOR EACH ROW EXECUTE FUNCTION require_joinable_tournament();
CREATE TRIGGER redemptions_require_joinable_tournament BEFORE INSERT ON invitation_redemptions
FOR EACH ROW EXECUTE FUNCTION require_joinable_tournament();
CREATE TRIGGER memberships_require_joinable_tournament BEFORE INSERT ON tournament_memberships
FOR EACH ROW EXECUTE FUNCTION require_joinable_tournament();
CREATE TRIGGER entrants_require_joinable_tournament BEFORE INSERT ON tournament_players
FOR EACH ROW EXECUTE FUNCTION require_joinable_tournament();
CREATE TRIGGER memberships_require_joinable_identity
BEFORE UPDATE OF tournament_id, user_id ON tournament_memberships
FOR EACH ROW WHEN (OLD.tournament_id IS DISTINCT FROM NEW.tournament_id OR OLD.user_id IS DISTINCT FROM NEW.user_id)
EXECUTE FUNCTION require_joinable_tournament();
CREATE TRIGGER entrants_require_joinable_identity
BEFORE UPDATE OF tournament_id, player_id ON tournament_players
FOR EACH ROW WHEN (OLD.tournament_id IS DISTINCT FROM NEW.tournament_id OR OLD.player_id IS DISTINCT FROM NEW.player_id)
EXECUTE FUNCTION require_joinable_tournament();

CREATE FUNCTION protect_closed_tournament_rounds() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE parent_status tournament_status;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        SELECT status INTO parent_status FROM tournaments WHERE id = OLD.tournament_id FOR SHARE;
        IF parent_status IN ('completed', 'archived') THEN
            RAISE EXCEPTION 'closed tournament round plans are immutable'
                USING ERRCODE = '23514', CONSTRAINT = 'closed_tournament_round_plan_locked';
        END IF;
    END IF;
    IF TG_OP <> 'DELETE' THEN
        SELECT status INTO parent_status FROM tournaments WHERE id = NEW.tournament_id FOR SHARE;
        IF parent_status IN ('completed', 'archived') THEN
            RAISE EXCEPTION 'closed tournament round plans are immutable'
                USING ERRCODE = '23514', CONSTRAINT = 'closed_tournament_round_plan_locked';
        END IF;
        RETURN NEW;
    END IF;
    RETURN OLD;
END;
$$;
CREATE TRIGGER rounds_protect_closed_plan
BEFORE INSERT OR DELETE OR UPDATE OF tournament_id ON rounds
FOR EACH ROW EXECUTE FUNCTION protect_closed_tournament_rounds();

COMMIT;
