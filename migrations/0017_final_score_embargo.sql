BEGIN;

ALTER TABLE rounds
    ADD COLUMN final_scores_hidden_until TIMESTAMPTZ;

CREATE FUNCTION protect_started_tournament_round_count() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.number_of_rounds IS DISTINCT FROM OLD.number_of_rounds
       AND (OLD.status IS DISTINCT FROM 'draft' OR NEW.status IS DISTINCT FROM 'draft') THEN
        RAISE EXCEPTION 'tournament round count is frozen when the tournament starts'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_round_count_started_frozen';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tournaments_protect_started_round_count
BEFORE UPDATE OF number_of_rounds ON tournaments
FOR EACH ROW EXECUTE FUNCTION protect_started_tournament_round_count();

CREATE FUNCTION protect_started_round_number() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    parent_status tournament_status;
BEGIN
    IF NEW.round_number IS NOT DISTINCT FROM OLD.round_number THEN
        RETURN NEW;
    END IF;

    -- The UPDATE already owns this round row. Locking its parent second matches
    -- tournament start's deterministic rounds-before-tournament order.
    SELECT status INTO parent_status
    FROM tournaments
    WHERE id = OLD.tournament_id
    FOR SHARE;

    IF parent_status IS DISTINCT FROM 'draft' THEN
        RAISE EXCEPTION 'round number is frozen when the tournament starts'
            USING ERRCODE = '23514', CONSTRAINT = 'round_number_started_frozen';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER rounds_protect_started_round_number
BEFORE UPDATE OF round_number ON rounds
FOR EACH ROW EXECUTE FUNCTION protect_started_round_number();

-- Preserve the trusted confirmation clock for already-ready final rounds. This
-- runs before the write guard is installed so schema upgrades do not need a
-- synthetic workflow context.
UPDATE rounds AS target_round
SET final_scores_hidden_until = ready.latest_confirmation + INTERVAL '24 hours'
FROM (
    SELECT round.id, max(confirmation.confirmed_at) AS latest_confirmation
    FROM rounds AS round
    JOIN tournaments AS tournament ON tournament.id = round.tournament_id
    JOIN scorecard_confirmations AS confirmation ON confirmation.round_id = round.id
    WHERE round.round_number = tournament.number_of_rounds
      AND round_scorecards_ready(round.id)
    GROUP BY round.id
) AS ready
WHERE target_round.id = ready.id;

CREATE FUNCTION final_score_embargo_is_unexpired(
    deadline TIMESTAMPTZ,
    observed_at TIMESTAMPTZ
) RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
    SELECT deadline > observed_at;
$$;

CREATE FUNCTION protect_final_score_embargo() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    workflow_round_id TEXT;
    workflow_time TIMESTAMPTZ;
    is_final_round BOOLEAN;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.final_scores_hidden_until IS NOT NULL THEN
            RAISE EXCEPTION 'final-score embargo must be changed by the confirmation workflow'
                USING ERRCODE = '23514', CONSTRAINT = 'final_score_embargo_context_required';
        END IF;
        RETURN NEW;
    END IF;

    IF NEW.final_scores_hidden_until IS NOT DISTINCT FROM OLD.final_scores_hidden_until THEN
        RETURN NEW;
    END IF;

    workflow_round_id = current_setting('app.final_score_embargo_round_id', true);
    BEGIN
        workflow_time = current_setting('app.final_score_embargo_time', true)::TIMESTAMPTZ;
    EXCEPTION
        WHEN invalid_text_representation OR invalid_datetime_format OR null_value_not_allowed THEN
            workflow_time = NULL;
    END;

    -- Trigger nesting proves this update originated inside the confirmation
    -- trigger. The transaction-local facts bind it to that exact round and time.
    IF pg_trigger_depth() < 2
       OR workflow_round_id IS DISTINCT FROM OLD.id::TEXT
       OR workflow_time IS NULL THEN
        RAISE EXCEPTION 'final-score embargo must be changed by the confirmation workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'final_score_embargo_context_required';
    END IF;

    SELECT OLD.round_number = tournament.number_of_rounds
    INTO is_final_round
    FROM tournaments AS tournament
    WHERE tournament.id = OLD.tournament_id;

    IF NEW.final_scores_hidden_until IS NOT NULL THEN
        IF OLD.final_scores_hidden_until IS NOT NULL
           OR NOT is_final_round
           OR NOT round_scorecards_ready(OLD.id)
           OR NEW.final_scores_hidden_until IS DISTINCT FROM workflow_time + INTERVAL '24 hours' THEN
            RAISE EXCEPTION 'final-score embargo can only start for a ready final round'
                USING ERRCODE = '23514', CONSTRAINT = 'final_score_embargo_start_invalid';
        END IF;
    ELSIF OLD.final_scores_hidden_until IS NULL
       OR NOT final_score_embargo_is_unexpired(OLD.final_scores_hidden_until, workflow_time)
       OR round_scorecards_ready(OLD.id) THEN
        RAISE EXCEPTION 'final-score embargo can only clear after a pre-expiry confirmation reset'
            USING ERRCODE = '23514', CONSTRAINT = 'final_score_embargo_clear_invalid';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER rounds_protect_final_score_embargo
BEFORE INSERT OR UPDATE OF final_scores_hidden_until ON rounds
FOR EACH ROW EXECUTE FUNCTION protect_final_score_embargo();

CREATE FUNCTION maintain_final_score_embargo() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_round_id UUID;
    workflow_time TIMESTAMPTZ;
BEGIN
    target_round_id = CASE WHEN TG_OP = 'INSERT' THEN NEW.round_id ELSE OLD.round_id END;

    -- Parent cascades already own deletion. In that path the round is gone and
    -- there is no clock left to maintain.
    IF TG_OP = 'DELETE' AND NOT EXISTS (SELECT 1 FROM rounds WHERE id = target_round_id) THEN
        RETURN OLD;
    END IF;

    workflow_time = clock_timestamp();
    PERFORM set_config('app.final_score_embargo_round_id', target_round_id::TEXT, true);
    PERFORM set_config('app.final_score_embargo_time', workflow_time::TEXT, true);

    IF TG_OP = 'INSERT' THEN
        UPDATE rounds AS round
        SET final_scores_hidden_until = workflow_time + INTERVAL '24 hours'
        FROM tournaments AS tournament
        WHERE round.id = target_round_id
          AND tournament.id = round.tournament_id
          AND round.round_number = tournament.number_of_rounds
          AND round.final_scores_hidden_until IS NULL
          AND round_scorecards_ready(round.id);
    ELSE
        UPDATE rounds
        SET final_scores_hidden_until = NULL
        WHERE id = target_round_id
          AND final_score_embargo_is_unexpired(final_scores_hidden_until, workflow_time);
    END IF;

    PERFORM set_config('app.final_score_embargo_round_id', '', true);
    PERFORM set_config('app.final_score_embargo_time', '', true);
    RETURN CASE WHEN TG_OP = 'INSERT' THEN NEW ELSE OLD END;
END;
$$;

CREATE TRIGGER scorecard_confirmations_maintain_final_score_embargo
AFTER INSERT OR DELETE ON scorecard_confirmations
FOR EACH ROW EXECUTE FUNCTION maintain_final_score_embargo();

COMMIT;
