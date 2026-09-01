BEGIN;

-- Before this lifecycle existed, a round could legally be opened while its
-- parent tournament remained draft. Preserve those rounds exactly and promote
-- only their legacy parent tournaments. The normal tournaments updated_at
-- trigger intentionally records this schema-upgrade lifecycle correction.
UPDATE tournaments AS tournament
SET status = 'active'
WHERE tournament.status = 'draft'
  AND EXISTS (
      SELECT 1
      FROM rounds
      WHERE rounds.tournament_id = tournament.id
        AND rounds.status <> 'draft'
  );

CREATE FUNCTION require_draft_tournament_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.status IS DISTINCT FROM 'draft' THEN
        RAISE EXCEPTION 'new tournaments must begin in draft'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_insert_requires_draft';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tournaments_require_draft_insert
BEFORE INSERT ON tournaments
FOR EACH ROW EXECUTE FUNCTION require_draft_tournament_insert();

CREATE FUNCTION validate_tournament_status_transition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    actor_text TEXT;
    actor_id UUID;
    stored_round_count BIGINT;
    draft_round_count BIGINT;
    first_round_number SMALLINT;
    last_round_number SMALLINT;
BEGIN
    IF NEW.status IS NOT DISTINCT FROM OLD.status THEN
        RETURN NEW;
    END IF;

    IF OLD.status = 'draft' AND NEW.status = 'active' THEN
        IF current_setting('app.tournament_start_tournament_id', true)
               IS DISTINCT FROM OLD.id::TEXT THEN
            RAISE EXCEPTION 'tournaments must be started through the lifecycle workflow'
                USING ERRCODE = '23514', CONSTRAINT = 'tournament_start_context_required';
        END IF;

        actor_text = current_setting('app.tournament_start_user_id', true);
        BEGIN
            actor_id = actor_text::UUID;
        EXCEPTION WHEN invalid_text_representation THEN
            RAISE EXCEPTION 'tournament start requires an exact tournament administrator'
                USING ERRCODE = '23514', CONSTRAINT = 'tournament_start_admin_required';
        END;

        IF NOT EXISTS (
            SELECT 1
            FROM tournament_memberships
            WHERE tournament_id = OLD.id AND user_id = actor_id AND role = 'admin'
        ) THEN
            RAISE EXCEPTION 'tournament start requires an exact tournament administrator'
                USING ERRCODE = '23514', CONSTRAINT = 'tournament_start_admin_required';
        END IF;

        SELECT count(*), count(*) FILTER (WHERE status = 'draft'), min(round_number),
               max(round_number)
        INTO stored_round_count, draft_round_count, first_round_number, last_round_number
        FROM rounds
        WHERE tournament_id = OLD.id;

        IF stored_round_count <> OLD.number_of_rounds
           OR draft_round_count <> stored_round_count
           OR first_round_number IS DISTINCT FROM 1
           OR last_round_number IS DISTINCT FROM OLD.number_of_rounds THEN
            RAISE EXCEPTION 'tournament start requires the complete draft round plan'
                USING ERRCODE = '23514', CONSTRAINT = 'tournament_start_round_plan_not_ready';
        END IF;

        IF NOT EXISTS (
            SELECT 1
            FROM tournament_players tp
            JOIN players p ON p.id = tp.player_id
            WHERE tp.tournament_id = OLD.id AND tp.status = 'active' AND p.active
        ) THEN
            RAISE EXCEPTION 'tournament start requires at least one active entrant'
                USING ERRCODE = '23514', CONSTRAINT = 'tournament_start_entrant_not_ready';
        END IF;
    ELSIF (OLD.status = 'active' AND NEW.status = 'completed')
       OR (OLD.status = 'completed' AND NEW.status = 'archived') THEN
        RETURN NEW;
    ELSE
        RAISE EXCEPTION 'tournament status transition is not allowed'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_status_transition_invalid';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tournaments_validate_status_transition
BEFORE UPDATE OF status ON tournaments
FOR EACH ROW EXECUTE FUNCTION validate_tournament_status_transition();

-- Round opening is a separate lifecycle action and is only valid after the
-- parent tournament has passed the guarded start transition above.
CREATE OR REPLACE FUNCTION validate_round_lifecycle_transition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    opening_round_id TEXT;
    required_snapshot_count BIGINT;
    captured_snapshot_count BIGINT;
    required_team_snapshot_count BIGINT;
    captured_team_snapshot_count BIGINT;
    parent_tournament_status tournament_status;
BEGIN
    IF NEW.status IS NOT DISTINCT FROM OLD.status THEN
        RETURN NEW;
    END IF;

    IF NOT ((OLD.status = 'draft' AND NEW.status = 'open')
            OR (OLD.status = 'open' AND NEW.status = 'completed')
            OR (OLD.status = 'completed' AND NEW.status = 'locked')) THEN
        RAISE EXCEPTION 'round status transition is not allowed'
            USING ERRCODE = '23514', CONSTRAINT = 'round_status_transition_invalid';
    END IF;

    IF OLD.status = 'draft' THEN
        opening_round_id = current_setting('app.round_opening_id', true);
        IF opening_round_id IS DISTINCT FROM OLD.id::TEXT THEN
            RAISE EXCEPTION 'rounds must be opened through the lifecycle workflow'
                USING ERRCODE = '23514', CONSTRAINT = 'round_opening_context_required';
        END IF;

        SELECT status INTO parent_tournament_status
        FROM tournaments
        WHERE id = OLD.tournament_id;
        IF parent_tournament_status IS DISTINCT FROM 'active' THEN
            RAISE EXCEPTION 'round opening requires an active tournament'
                USING ERRCODE = '23514', CONSTRAINT = 'round_opening_tournament_inactive';
        END IF;

        SELECT count(*) INTO required_snapshot_count
        FROM tournament_players tp
        JOIN players p ON p.id = tp.player_id
        WHERE tp.tournament_id = OLD.tournament_id
          AND tp.status = 'active'
          AND p.active;

        SELECT count(*) INTO captured_snapshot_count
        FROM round_handicap_snapshots
        WHERE round_id = OLD.id;

        IF captured_snapshot_count <> required_snapshot_count THEN
            RAISE EXCEPTION 'round opening requires one snapshot per active entrant'
                USING ERRCODE = '23514', CONSTRAINT = 'round_opening_snapshots_incomplete';
        END IF;
        IF OLD.scoring_format::TEXT = 'two_player_foursomes' THEN
            SELECT count(*) INTO required_team_snapshot_count
            FROM teams WHERE round_id = OLD.id;
            SELECT count(*) INTO captured_team_snapshot_count
            FROM round_team_handicap_snapshots WHERE round_id = OLD.id;
            IF required_team_snapshot_count = 0
               OR captured_team_snapshot_count <> required_team_snapshot_count THEN
                RAISE EXCEPTION 'foursomes opening requires one handicap snapshot per team'
                    USING ERRCODE = '23514', CONSTRAINT = 'round_opening_team_snapshots_incomplete';
            END IF;
        END IF;
    ELSIF OLD.status = 'open' THEN
        IF current_setting('app.round_completion_id', true) IS DISTINCT FROM OLD.id::TEXT THEN
            RAISE EXCEPTION 'rounds must be completed through the completion workflow'
                USING ERRCODE = '23514', CONSTRAINT = 'round_completion_context_required';
        END IF;
        IF NOT round_scorecards_ready(OLD.id) THEN
            RAISE EXCEPTION 'round completion requires complete confirmed scorecards'
                USING ERRCODE = '23514', CONSTRAINT = 'round_scorecards_not_ready';
        END IF;
    ELSIF OLD.status = 'completed' THEN
        IF current_setting('app.round_lock_id', true) IS DISTINCT FROM OLD.id::TEXT THEN
            RAISE EXCEPTION 'rounds must be locked through the locking workflow'
                USING ERRCODE = '23514', CONSTRAINT = 'round_lock_context_required';
        END IF;
        IF NOT round_scorecards_ready(OLD.id) THEN
            RAISE EXCEPTION 'round locking requires complete confirmed scorecards'
                USING ERRCODE = '23514', CONSTRAINT = 'round_scorecards_not_ready';
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

COMMIT;
