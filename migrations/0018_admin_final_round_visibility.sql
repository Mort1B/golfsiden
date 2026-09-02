BEGIN;

ALTER TABLE tournaments
    ADD COLUMN final_round_back_nine_hidden BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN visibility_updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp();

-- Preserve the projection users saw immediately before this migration. Only an
-- expired completed or locked final was visible under schema 17; every other
-- existing tournament keeps the new fail-closed default.
UPDATE tournaments AS tournament
SET final_round_back_nine_hidden = FALSE,
    visibility_updated_at = transaction_timestamp()
WHERE EXISTS (
    SELECT 1
    FROM rounds AS final_round
    WHERE final_round.tournament_id = tournament.id
      AND final_round.round_number = tournament.number_of_rounds
      AND final_round.status IN ('completed', 'locked')
      AND final_round.final_scores_hidden_until IS NOT NULL
      AND final_round.final_scores_hidden_until <= transaction_timestamp()
);

DROP TRIGGER scorecard_confirmations_maintain_final_score_embargo
    ON scorecard_confirmations;
DROP FUNCTION maintain_final_score_embargo();
DROP TRIGGER rounds_protect_final_score_embargo ON rounds;
DROP FUNCTION protect_final_score_embargo();
DROP FUNCTION final_score_embargo_is_unexpired(TIMESTAMPTZ, TIMESTAMPTZ);

ALTER TABLE rounds DROP COLUMN final_scores_hidden_until;

CREATE FUNCTION protect_final_round_visibility() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    session_text TEXT;
    session_id UUID;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.final_round_back_nine_hidden IS DISTINCT FROM TRUE THEN
            RAISE EXCEPTION 'new tournaments must hide the final back nine'
                USING ERRCODE = '23514', CONSTRAINT = 'final_round_visibility_default_hidden';
        END IF;
        NEW.visibility_updated_at = clock_timestamp();
        RETURN NEW;
    END IF;

    IF NEW.final_round_back_nine_hidden IS NOT DISTINCT FROM OLD.final_round_back_nine_hidden
       AND NEW.visibility_updated_at IS NOT DISTINCT FROM OLD.visibility_updated_at THEN
        RETURN NEW;
    END IF;

    IF NEW.final_round_back_nine_hidden IS NOT DISTINCT FROM OLD.final_round_back_nine_hidden THEN
        RAISE EXCEPTION 'final-round visibility timestamp is workflow-managed'
            USING ERRCODE = '23514', CONSTRAINT = 'final_round_visibility_timestamp_managed';
    END IF;

    IF current_setting('app.final_round_visibility_tournament_id', true)
           IS DISTINCT FROM OLD.id::TEXT THEN
        RAISE EXCEPTION 'final-round visibility must use the visibility workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'final_round_visibility_context_required';
    END IF;

    session_text = current_setting('app.final_round_visibility_session_id', true);
    BEGIN
        session_id = session_text::UUID;
    EXCEPTION WHEN invalid_text_representation THEN
        RAISE EXCEPTION 'final-round visibility requires an active tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'final_round_visibility_admin_required';
    END;

    IF NOT EXISTS (
        SELECT 1
        FROM user_sessions AS session
        JOIN tournament_memberships AS membership
          ON membership.user_id = session.user_id
         AND membership.tournament_id = OLD.id
         AND membership.role = 'admin'
        WHERE session.id = session_id
          AND session.revoked_at IS NULL
          AND session.expires_at > clock_timestamp()
    ) THEN
        RAISE EXCEPTION 'final-round visibility requires an active tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'final_round_visibility_admin_required';
    END IF;

    NEW.visibility_updated_at = GREATEST(
        clock_timestamp(),
        OLD.visibility_updated_at + INTERVAL '1 microsecond'
    );
    RETURN NEW;
END;
$$;

CREATE TRIGGER tournaments_protect_final_round_visibility
BEFORE INSERT OR UPDATE OF final_round_back_nine_hidden, visibility_updated_at ON tournaments
FOR EACH ROW EXECUTE FUNCTION protect_final_round_visibility();

COMMIT;
