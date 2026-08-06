DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM scores s
        JOIN rounds r ON r.id = s.round_id
        LEFT JOIN holes h ON h.id = s.hole_id AND h.tee_id = r.tee_id
        LEFT JOIN round_handicap_snapshots rhs
            ON rhs.round_id = s.round_id AND rhs.player_id = s.player_id
        WHERE h.id IS NULL
           OR (r.scoring_format = 'individual_stroke_play'
               AND (s.player_id IS NULL OR s.team_id IS NOT NULL OR rhs.player_id IS NULL))
           OR (r.scoring_format = 'team_scramble'
               AND (s.team_id IS NULL OR s.player_id IS NOT NULL))
           OR r.status = 'draft'
           OR s.confirmed
           OR s.locked
    ) THEN
        RAISE EXCEPTION 'migration blocked: existing scores violate scorecard ownership, hole, lifecycle, or legacy state rules';
    END IF;
END;
$$;

ALTER TABLE scores
    DROP CONSTRAINT scores_tournament_id_player_id_fkey,
    ADD CONSTRAINT scores_tournament_id_player_id_fkey
        FOREIGN KEY (tournament_id, player_id)
        REFERENCES tournament_players(tournament_id, player_id)
        ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT scores_round_player_snapshot_fkey
        FOREIGN KEY (round_id, player_id)
        REFERENCES round_handicap_snapshots(round_id, player_id)
        ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE score_audits
    DROP CONSTRAINT score_audits_score_id_fkey,
    ADD CONSTRAINT score_audits_score_id_fkey
        FOREIGN KEY (score_id) REFERENCES scores(id) ON DELETE CASCADE;

CREATE TABLE scorecard_confirmations (
    id UUID PRIMARY KEY,
    round_id UUID NOT NULL,
    tournament_id UUID NOT NULL,
    player_id UUID,
    team_id UUID,
    confirmed_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    confirmed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (round_id, tournament_id)
        REFERENCES rounds(id, tournament_id) ON DELETE CASCADE,
    FOREIGN KEY (round_id, player_id)
        REFERENCES round_handicap_snapshots(round_id, player_id)
        ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (team_id, round_id, tournament_id)
        REFERENCES teams(id, round_id, tournament_id)
        ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT scorecard_confirmation_owner_check
        CHECK ((player_id IS NOT NULL)::integer + (team_id IS NOT NULL)::integer = 1)
);
CREATE UNIQUE INDEX scorecard_confirmations_player_idx
    ON scorecard_confirmations(round_id, player_id) WHERE player_id IS NOT NULL;
CREATE UNIQUE INDEX scorecard_confirmations_team_idx
    ON scorecard_confirmations(round_id, team_id) WHERE team_id IS NOT NULL;

DROP TRIGGER scores_protect_locked_round ON scores;
DROP FUNCTION protect_locked_round_score();

CREATE FUNCTION acquire_score_round_lock(target_round_id UUID) RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    PERFORM id FROM rounds WHERE id = target_round_id FOR UPDATE NOWAIT;
EXCEPTION
    WHEN lock_not_available THEN
        RAISE EXCEPTION 'score mutation could not acquire the round lock'
            USING ERRCODE = '23514', CONSTRAINT = 'score_round_lock_required';
END;
$$;

CREATE FUNCTION validate_score_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_round_id UUID;
    parent_status round_status;
    parent_format scoring_format;
    parent_tee_id UUID;
BEGIN
    target_round_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.round_id ELSE NEW.round_id END;

    IF TG_OP = 'DELETE'
       AND NOT EXISTS (SELECT 1 FROM rounds WHERE id = target_round_id) THEN
        RETURN OLD;
    END IF;

    IF current_setting('app.score_mutation_round_id', true)
       IS DISTINCT FROM target_round_id::TEXT THEN
        RAISE EXCEPTION 'scores must be changed through the score workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'score_mutation_context_required';
    END IF;

    PERFORM acquire_score_round_lock(target_round_id);

    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'scores cannot be deleted while their round exists'
            USING ERRCODE = '23514', CONSTRAINT = 'score_delete_forbidden';
    END IF;

    SELECT status, scoring_format, tee_id
    INTO parent_status, parent_format, parent_tee_id
    FROM rounds WHERE id = target_round_id;

    IF parent_status NOT IN ('open', 'completed')
       AND NOT (parent_status = 'locked'
                AND current_setting('app.admin_correction', true) = 'true') THEN
        RAISE EXCEPTION 'round is not open for score changes'
            USING ERRCODE = '23514', CONSTRAINT = 'score_round_not_editable';
    END IF;

    IF TG_OP = 'UPDATE' THEN
        IF NEW.id IS DISTINCT FROM OLD.id
           OR NEW.round_id IS DISTINCT FROM OLD.round_id
           OR NEW.tournament_id IS DISTINCT FROM OLD.tournament_id
           OR NEW.hole_id IS DISTINCT FROM OLD.hole_id
           OR NEW.player_id IS DISTINCT FROM OLD.player_id
           OR NEW.team_id IS DISTINCT FROM OLD.team_id
           OR NEW.submitted_at IS DISTINCT FROM OLD.submitted_at
           OR NEW.confirmed IS DISTINCT FROM OLD.confirmed
           OR NEW.locked IS DISTINCT FROM OLD.locked THEN
            RAISE EXCEPTION 'score identity is immutable'
                USING ERRCODE = '23514', CONSTRAINT = 'score_identity_immutable';
        END IF;
        IF NEW.gross_strokes IS NOT DISTINCT FROM OLD.gross_strokes
           AND NEW.submitted_by IS DISTINCT FROM OLD.submitted_by THEN
            RAISE EXCEPTION 'unchanged scores cannot replace their submitter'
                USING ERRCODE = '23514', CONSTRAINT = 'score_unchanged_submitter';
        END IF;
        IF NEW.gross_strokes IS DISTINCT FROM OLD.gross_strokes THEN
            NEW.confirmed = FALSE;
        END IF;
    ELSE
        NEW.confirmed = FALSE;
        NEW.locked = FALSE;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM holes WHERE id = NEW.hole_id AND tee_id = parent_tee_id
    ) THEN
        RAISE EXCEPTION 'hole does not belong to the round tee'
            USING ERRCODE = '23514', CONSTRAINT = 'score_hole_not_in_round';
    END IF;

    IF (parent_format = 'individual_stroke_play'
        AND (NEW.player_id IS NULL OR NEW.team_id IS NOT NULL))
       OR (parent_format = 'team_scramble'
           AND (NEW.team_id IS NULL OR NEW.player_id IS NOT NULL)) THEN
        RAISE EXCEPTION 'score owner does not match the round format'
            USING ERRCODE = '23514', CONSTRAINT = 'score_owner_format_mismatch';
    END IF;

    IF NEW.player_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM round_handicap_snapshots
        WHERE round_id = NEW.round_id AND player_id = NEW.player_id
    ) THEN
        RAISE EXCEPTION 'player was not snapshotted for this round'
            USING ERRCODE = '23514', CONSTRAINT = 'score_owner_ineligible';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER scores_validate_mutation
BEFORE INSERT OR UPDATE OR DELETE ON scores
FOR EACH ROW EXECUTE FUNCTION validate_score_mutation();

CREATE FUNCTION invalidate_scorecard_confirmation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' OR NEW.gross_strokes IS DISTINCT FROM OLD.gross_strokes THEN
        DELETE FROM scorecard_confirmations
        WHERE round_id = NEW.round_id
          AND ((NEW.player_id IS NOT NULL AND player_id = NEW.player_id)
               OR (NEW.team_id IS NOT NULL AND team_id = NEW.team_id));
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER scores_invalidate_confirmation
AFTER INSERT OR UPDATE ON scores
FOR EACH ROW EXECUTE FUNCTION invalidate_scorecard_confirmation();

CREATE FUNCTION validate_scorecard_confirmation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_round_id UUID;
    target_player_id UUID;
    target_team_id UUID;
    parent_status round_status;
    parent_format scoring_format;
    required_holes SMALLINT;
    scored_holes BIGINT;
BEGIN
    target_round_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.round_id ELSE NEW.round_id END;
    target_player_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.player_id ELSE NEW.player_id END;
    target_team_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.team_id ELSE NEW.team_id END;

    IF TG_OP = 'DELETE'
       AND NOT EXISTS (SELECT 1 FROM rounds WHERE id = target_round_id) THEN
        RETURN OLD;
    END IF;

    IF current_setting('app.score_mutation_round_id', true)
       IS DISTINCT FROM target_round_id::TEXT THEN
        RAISE EXCEPTION 'scorecard confirmation must use the score workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'score_confirmation_context_required';
    END IF;

    PERFORM acquire_score_round_lock(target_round_id);

    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'scorecard confirmations are immutable'
            USING ERRCODE = '23514', CONSTRAINT = 'score_confirmation_immutable';
    END IF;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;

    SELECT status, scoring_format, number_of_holes
    INTO parent_status, parent_format, required_holes
    FROM rounds WHERE id = target_round_id;

    IF parent_status NOT IN ('open', 'completed')
       AND NOT (parent_status = 'locked'
                AND current_setting('app.admin_correction', true) = 'true') THEN
        RAISE EXCEPTION 'round is not open for scorecard confirmation'
            USING ERRCODE = '23514', CONSTRAINT = 'score_round_not_editable';
    END IF;

    IF (parent_format = 'individual_stroke_play'
        AND (target_player_id IS NULL OR target_team_id IS NOT NULL))
       OR (parent_format = 'team_scramble'
           AND (target_team_id IS NULL OR target_player_id IS NOT NULL)) THEN
        RAISE EXCEPTION 'scorecard owner does not match the round format'
            USING ERRCODE = '23514', CONSTRAINT = 'score_owner_format_mismatch';
    END IF;

    SELECT count(*) INTO scored_holes
    FROM scores
    WHERE round_id = target_round_id
      AND ((target_player_id IS NOT NULL AND player_id = target_player_id)
           OR (target_team_id IS NOT NULL AND team_id = target_team_id));

    IF scored_holes <> required_holes THEN
        RAISE EXCEPTION 'scorecard is incomplete'
            USING ERRCODE = '23514', CONSTRAINT = 'scorecard_incomplete';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER scorecard_confirmations_validate
BEFORE INSERT OR UPDATE OR DELETE ON scorecard_confirmations
FOR EACH ROW EXECUTE FUNCTION validate_scorecard_confirmation();

CREATE FUNCTION protect_score_audit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    audit_round_id UUID;
BEGIN
    IF TG_OP = 'INSERT' THEN
        SELECT round_id INTO audit_round_id FROM scores WHERE id = NEW.score_id;
        IF current_setting('app.score_mutation_round_id', true)
           IS DISTINCT FROM audit_round_id::TEXT THEN
            RAISE EXCEPTION 'score audits must be created by the score workflow'
                USING ERRCODE = '23514', CONSTRAINT = 'score_audit_context_required';
        END IF;
        RETURN NEW;
    END IF;

    IF TG_OP = 'UPDATE' OR EXISTS (SELECT 1 FROM scores WHERE id = OLD.score_id) THEN
        RAISE EXCEPTION 'score audit history is append-only'
            USING ERRCODE = '23514', CONSTRAINT = 'score_audit_immutable';
    END IF;
    RETURN OLD;
END;
$$;

CREATE TRIGGER score_audits_protect
BEFORE INSERT OR UPDATE OR DELETE ON score_audits
FOR EACH ROW EXECUTE FUNCTION protect_score_audit();
