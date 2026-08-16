ALTER TABLE users ADD COLUMN username TEXT;

-- Preserve the recognizable email local part where possible. Processing in a
-- stable order and probing suffixes makes every collision deterministic,
-- including local parts which already end in a numeric suffix.
DO $$
DECLARE
    legacy RECORD;
    base_username TEXT;
    candidate TEXT;
    suffix_number INTEGER;
    suffix TEXT;
BEGIN
    FOR legacy IN
        SELECT id, email
        FROM users
        ORDER BY lower(email), email, id
    LOOP
        base_username := regexp_replace(
            lower(split_part(legacy.email, '@', 1)),
            '[^a-z0-9_-]+',
            '_',
            'g'
        );
        base_username := trim(BOTH '_-' FROM base_username);
        IF length(base_username) < 3 THEN
            base_username := 'user_' || base_username;
        END IF;
        base_username := left(base_username, 32);
        candidate := base_username;
        suffix_number := 2;

        WHILE EXISTS (SELECT 1 FROM users WHERE username = candidate) LOOP
            suffix := '_' || suffix_number::TEXT;
            candidate := left(base_username, 32 - length(suffix)) || suffix;
            suffix_number := suffix_number + 1;
        END LOOP;

        UPDATE users SET username = candidate WHERE id = legacy.id;
    END LOOP;
END
$$;

ALTER TABLE users
    ALTER COLUMN username SET NOT NULL,
    ADD CONSTRAINT users_username_syntax_check
        CHECK (username ~ '^[a-z0-9_-]{3,32}$');

CREATE UNIQUE INDEX users_username_normalized_idx ON users (lower(username));

DROP INDEX users_email_normalized_idx;
ALTER TABLE users DROP CONSTRAINT users_email_key;
ALTER TABLE users DROP COLUMN email;

CREATE TYPE tournament_handicap_lock_reason AS ENUM (
    'round_opened',
    'snapshot_captured'
);

CREATE TABLE tournament_handicap_locks (
    tournament_id UUID PRIMARY KEY
        REFERENCES tournaments(id) ON DELETE CASCADE,
    reason tournament_handicap_lock_reason NOT NULL,
    locked_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The marker survives round and snapshot deletion and therefore records the
-- historical fact required by the correction policy.
INSERT INTO tournament_handicap_locks (tournament_id, reason, locked_at)
SELECT t.id,
       CASE WHEN EXISTS (
           SELECT 1 FROM round_handicap_snapshots rhs
           WHERE rhs.tournament_id = t.id
       ) THEN 'snapshot_captured'::tournament_handicap_lock_reason
       ELSE 'round_opened'::tournament_handicap_lock_reason
       END,
       COALESCE((
           SELECT min(rhs.captured_at)
           FROM round_handicap_snapshots rhs
           WHERE rhs.tournament_id = t.id
       ), now())
FROM tournaments t
WHERE EXISTS (
    SELECT 1 FROM round_handicap_snapshots rhs
    WHERE rhs.tournament_id = t.id
) OR EXISTS (
    SELECT 1 FROM rounds r
    WHERE r.tournament_id = t.id AND r.status <> 'draft'
);

CREATE FUNCTION capture_tournament_handicap_lock() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO tournament_handicap_locks (tournament_id, reason, locked_at)
    VALUES (NEW.tournament_id, 'snapshot_captured', NEW.captured_at)
    ON CONFLICT (tournament_id) DO NOTHING;
    RETURN NEW;
END;
$$;

CREATE TRIGGER round_handicap_snapshots_lock_tournament_handicap
AFTER INSERT ON round_handicap_snapshots
FOR EACH ROW EXECUTE FUNCTION capture_tournament_handicap_lock();

CREATE FUNCTION protect_tournament_handicap_lock() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' OR EXISTS (
        SELECT 1 FROM tournaments WHERE id = OLD.tournament_id
    ) THEN
        RAISE EXCEPTION 'tournament handicap lock history is immutable'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_handicap_lock_immutable';
    END IF;
    RETURN OLD;
END;
$$;

CREATE TRIGGER tournament_handicap_locks_protect
BEFORE UPDATE OR DELETE ON tournament_handicap_locks
FOR EACH ROW EXECUTE FUNCTION protect_tournament_handicap_lock();

CREATE FUNCTION protect_tournament_handicap_change() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    correction_user_id UUID;
    correction_reason TEXT;
BEGIN
    IF NEW.tournament_handicap IS NOT DISTINCT FROM OLD.tournament_handicap THEN
        RAISE EXCEPTION 'tournament handicap is unchanged'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_handicap_unchanged';
    END IF;

    IF current_setting('app.tournament_handicap_correction_tournament_id', true)
            IS DISTINCT FROM OLD.tournament_id::TEXT
       OR current_setting('app.tournament_handicap_correction_player_id', true)
            IS DISTINCT FROM OLD.player_id::TEXT THEN
        RAISE EXCEPTION 'tournament handicaps require the correction workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_handicap_correction_context_required';
    END IF;

    correction_user_id := current_setting(
        'app.tournament_handicap_correction_user_id', true
    )::UUID;
    correction_reason := current_setting(
        'app.tournament_handicap_correction_reason', true
    );

    IF correction_reason IS NULL
       OR btrim(correction_reason) = ''
       OR octet_length(correction_reason) > 500 THEN
        RAISE EXCEPTION 'tournament handicap corrections require a bounded reason'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_handicap_reason_invalid';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM tournament_memberships
        WHERE tournament_id = OLD.tournament_id
          AND user_id = correction_user_id
          AND role = 'admin'
    ) THEN
        RAISE EXCEPTION 'tournament handicap corrections require a tournament admin'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_handicap_admin_required';
    END IF;

    BEGIN
        PERFORM id
        FROM rounds
        WHERE tournament_id = OLD.tournament_id
        ORDER BY id
        FOR UPDATE NOWAIT;
    EXCEPTION WHEN lock_not_available THEN
        RAISE EXCEPTION 'tournament handicap correction could not acquire round locks'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_handicap_lock_required';
    END;

    IF EXISTS (
        SELECT 1 FROM tournament_handicap_locks
        WHERE tournament_id = OLD.tournament_id
    ) OR EXISTS (
        SELECT 1 FROM rounds
        WHERE tournament_id = OLD.tournament_id AND status <> 'draft'
    ) OR EXISTS (
        SELECT 1 FROM round_handicap_snapshots
        WHERE tournament_id = OLD.tournament_id
    ) THEN
        RAISE EXCEPTION 'tournament handicap is locked after round opening'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_handicap_locked';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tournament_players_protect_handicap_change
BEFORE UPDATE OF tournament_handicap ON tournament_players
FOR EACH ROW EXECUTE FUNCTION protect_tournament_handicap_change();

CREATE FUNCTION audit_tournament_handicap_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO tournament_handicap_history (
        id,
        tournament_id,
        player_id,
        handicap_index,
        changed_by,
        reason
    ) VALUES (
        current_setting('app.tournament_handicap_correction_audit_id')::UUID,
        NEW.tournament_id,
        NEW.player_id,
        NEW.tournament_handicap,
        current_setting('app.tournament_handicap_correction_user_id')::UUID,
        btrim(current_setting('app.tournament_handicap_correction_reason'))
    );
    RETURN NEW;
END;
$$;

CREATE TRIGGER tournament_players_audit_handicap_change
AFTER UPDATE OF tournament_handicap ON tournament_players
FOR EACH ROW EXECUTE FUNCTION audit_tournament_handicap_change();

CREATE FUNCTION protect_tournament_handicap_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' OR EXISTS (
        SELECT 1 FROM tournaments WHERE id = OLD.tournament_id
    ) THEN
        RAISE EXCEPTION 'tournament handicap history is append-only'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_handicap_history_immutable';
    END IF;
    RETURN OLD;
END;
$$;

CREATE TRIGGER tournament_handicap_history_protect
BEFORE UPDATE OR DELETE ON tournament_handicap_history
FOR EACH ROW EXECUTE FUNCTION protect_tournament_handicap_history();
