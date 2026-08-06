CREATE FUNCTION round_scorecards_ready(target_round_id UUID) RETURNS BOOLEAN
LANGUAGE plpgsql STABLE AS $$
DECLARE
    target_format scoring_format;
    required_holes SMALLINT;
    owner_count BIGINT;
    invalid_owner_count BIGINT;
BEGIN
    SELECT scoring_format, number_of_holes
    INTO target_format, required_holes
    FROM rounds WHERE id = target_round_id;

    IF target_format = 'individual_stroke_play' THEN
        SELECT count(*) INTO owner_count
        FROM round_handicap_snapshots WHERE round_id = target_round_id;

        SELECT count(*) INTO invalid_owner_count
        FROM round_handicap_snapshots rhs
        WHERE rhs.round_id = target_round_id
          AND (required_holes <> (
                   SELECT count(*) FROM scores s
                   WHERE s.round_id = target_round_id AND s.player_id = rhs.player_id
               )
               OR NOT EXISTS (
                   SELECT 1 FROM scorecard_confirmations sc
                   WHERE sc.round_id = target_round_id AND sc.player_id = rhs.player_id
               ));
    ELSIF target_format = 'team_scramble' THEN
        SELECT count(*) INTO owner_count
        FROM teams WHERE round_id = target_round_id;

        SELECT count(*) INTO invalid_owner_count
        FROM teams t
        WHERE t.round_id = target_round_id
          AND (required_holes <> (
                   SELECT count(*) FROM scores s
                   WHERE s.round_id = target_round_id AND s.team_id = t.id
               )
               OR NOT EXISTS (
                   SELECT 1 FROM scorecard_confirmations sc
                   WHERE sc.round_id = target_round_id AND sc.team_id = t.id
               ));
    ELSE
        RETURN FALSE;
    END IF;

    RETURN owner_count > 0 AND invalid_owner_count = 0;
END;
$$;

-- Freeze the rows read by the preflight until this transactional migration commits.
LOCK TABLE rounds, scores, scorecard_confirmations IN SHARE ROW EXCLUSIVE MODE;

DO $$
DECLARE
    invalid_rounds TEXT;
BEGIN
    SELECT string_agg(id::TEXT, ', ' ORDER BY id) INTO invalid_rounds
    FROM rounds
    WHERE status IN ('completed', 'locked')
      AND NOT round_scorecards_ready(id);

    IF invalid_rounds IS NOT NULL THEN
        RAISE EXCEPTION 'migration blocked: completed or locked rounds have incomplete or unconfirmed scorecards: %', invalid_rounds;
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION validate_round_lifecycle_transition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    opening_round_id TEXT;
    required_snapshot_count BIGINT;
    captured_snapshot_count BIGINT;
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
