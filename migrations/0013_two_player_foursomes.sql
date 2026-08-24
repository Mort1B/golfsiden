ALTER TYPE scoring_format ADD VALUE 'two_player_foursomes';

CREATE FUNCTION validate_foursomes_round_allowance() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.scoring_format::TEXT = 'two_player_foursomes'
       AND NEW.handicap_allowance_percent <> 50 THEN
        RAISE EXCEPTION 'two-player foursomes requires a 50 percent handicap allowance'
            USING ERRCODE = '23514', CONSTRAINT = 'round_foursomes_allowance_exact';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER rounds_validate_foursomes_allowance
BEFORE INSERT OR UPDATE OF scoring_format, handicap_allowance_percent ON rounds
FOR EACH ROW EXECUTE FUNCTION validate_foursomes_round_allowance();

CREATE TABLE round_team_handicap_snapshots (
    round_id UUID NOT NULL,
    tournament_id UUID NOT NULL,
    team_id UUID NOT NULL,
    playing_handicap SMALLINT NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT round_team_handicap_snapshots_pkey PRIMARY KEY (round_id, team_id),
    CONSTRAINT round_team_handicap_snapshots_round_fkey
        FOREIGN KEY (round_id, tournament_id)
        REFERENCES rounds(id, tournament_id) ON DELETE CASCADE,
    CONSTRAINT round_team_handicap_snapshots_team_fkey
        FOREIGN KEY (team_id, round_id, tournament_id)
        REFERENCES teams(id, round_id, tournament_id)
        ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED
);

CREATE FUNCTION protect_round_team_handicap_snapshot() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_round_id UUID;
    target_team_id UUID;
    parent_status round_status;
    parent_format TEXT;
    exact_member_count BIGINT;
    snapshotted_member_count BIGINT;
BEGIN
    target_round_id = CASE WHEN TG_OP = 'INSERT' THEN NEW.round_id ELSE OLD.round_id END;
    target_team_id = CASE WHEN TG_OP = 'INSERT' THEN NEW.team_id ELSE OLD.team_id END;
    SELECT status, scoring_format::TEXT INTO parent_status, parent_format
    FROM rounds WHERE id = target_round_id FOR UPDATE;

    IF TG_OP = 'INSERT' THEN
        IF parent_status IS DISTINCT FROM 'draft'
           OR parent_format IS DISTINCT FROM 'two_player_foursomes'
           OR current_setting('app.round_opening_id', true) IS DISTINCT FROM target_round_id::TEXT THEN
            RAISE EXCEPTION 'team handicap snapshots can only be captured while opening a foursomes round'
                USING ERRCODE = '23514', CONSTRAINT = 'round_team_snapshot_capture_frozen';
        END IF;
        SELECT count(*) INTO exact_member_count
        FROM team_memberships
        WHERE round_id = target_round_id AND team_id = target_team_id;
        SELECT count(*) INTO snapshotted_member_count
        FROM team_memberships tm
        JOIN round_handicap_snapshots rhs
          ON rhs.round_id = tm.round_id AND rhs.player_id = tm.player_id
        WHERE tm.round_id = target_round_id AND tm.team_id = target_team_id;
        IF exact_member_count <> 2 OR snapshotted_member_count <> 2 THEN
            RAISE EXCEPTION 'foursomes team snapshots require exactly two snapshotted members'
                USING ERRCODE = '23514', CONSTRAINT = 'round_team_snapshot_members_invalid';
        END IF;
    ELSIF parent_status IS NOT NULL THEN
        RAISE EXCEPTION 'round team handicap snapshots are immutable'
            USING ERRCODE = '23514', CONSTRAINT = 'round_team_snapshot_immutable';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

CREATE TRIGGER round_team_handicap_snapshots_protect
BEFORE INSERT OR UPDATE OR DELETE ON round_team_handicap_snapshots
FOR EACH ROW EXECUTE FUNCTION protect_round_team_handicap_snapshot();

CREATE OR REPLACE FUNCTION validate_score_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_round_id UUID;
    parent_status round_status;
    parent_format TEXT;
    parent_tee_id UUID;
BEGIN
    target_round_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.round_id ELSE NEW.round_id END;
    IF TG_OP = 'DELETE' AND NOT EXISTS (SELECT 1 FROM rounds WHERE id = target_round_id) THEN
        RETURN OLD;
    END IF;
    IF current_setting('app.score_mutation_round_id', true) IS DISTINCT FROM target_round_id::TEXT THEN
        RAISE EXCEPTION 'scores must be changed through the score workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'score_mutation_context_required';
    END IF;
    PERFORM acquire_score_round_lock(target_round_id);
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'scores cannot be deleted while their round exists'
            USING ERRCODE = '23514', CONSTRAINT = 'score_delete_forbidden';
    END IF;
    SELECT status, scoring_format::TEXT, tee_id
    INTO parent_status, parent_format, parent_tee_id FROM rounds WHERE id = target_round_id;
    IF parent_status NOT IN ('open', 'completed')
       AND NOT (parent_status = 'locked' AND current_setting('app.admin_correction', true) = 'true') THEN
        RAISE EXCEPTION 'round is not open for score changes'
            USING ERRCODE = '23514', CONSTRAINT = 'score_round_not_editable';
    END IF;
    IF TG_OP = 'UPDATE' THEN
        IF NEW.id IS DISTINCT FROM OLD.id OR NEW.round_id IS DISTINCT FROM OLD.round_id
           OR NEW.tournament_id IS DISTINCT FROM OLD.tournament_id
           OR NEW.hole_id IS DISTINCT FROM OLD.hole_id
           OR NEW.player_id IS DISTINCT FROM OLD.player_id OR NEW.team_id IS DISTINCT FROM OLD.team_id
           OR NEW.submitted_at IS DISTINCT FROM OLD.submitted_at
           OR NEW.confirmed IS DISTINCT FROM OLD.confirmed OR NEW.locked IS DISTINCT FROM OLD.locked THEN
            RAISE EXCEPTION 'score identity is immutable'
                USING ERRCODE = '23514', CONSTRAINT = 'score_identity_immutable';
        END IF;
        IF NEW.gross_strokes IS NOT DISTINCT FROM OLD.gross_strokes
           AND NEW.submitted_by IS DISTINCT FROM OLD.submitted_by THEN
            RAISE EXCEPTION 'unchanged scores cannot replace their submitter'
                USING ERRCODE = '23514', CONSTRAINT = 'score_unchanged_submitter';
        END IF;
        IF NEW.gross_strokes IS DISTINCT FROM OLD.gross_strokes THEN NEW.confirmed = FALSE; END IF;
    ELSE
        NEW.confirmed = FALSE;
        NEW.locked = FALSE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM holes WHERE id = NEW.hole_id AND tee_id = parent_tee_id) THEN
        RAISE EXCEPTION 'hole does not belong to the round tee'
            USING ERRCODE = '23514', CONSTRAINT = 'score_hole_not_in_round';
    END IF;
    IF (parent_format = 'individual_stroke_play' AND (NEW.player_id IS NULL OR NEW.team_id IS NOT NULL))
       OR (parent_format IN ('team_scramble', 'two_player_foursomes')
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
    IF parent_format = 'two_player_foursomes' AND NOT EXISTS (
        SELECT 1 FROM round_team_handicap_snapshots
        WHERE round_id = NEW.round_id AND team_id = NEW.team_id
    ) THEN
        RAISE EXCEPTION 'team was not snapshotted for this round'
            USING ERRCODE = '23514', CONSTRAINT = 'score_owner_ineligible';
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION validate_scorecard_confirmation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_round_id UUID;
    target_player_id UUID;
    target_team_id UUID;
    parent_status round_status;
    parent_format TEXT;
    required_holes SMALLINT;
    scored_holes BIGINT;
BEGIN
    target_round_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.round_id ELSE NEW.round_id END;
    target_player_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.player_id ELSE NEW.player_id END;
    target_team_id = CASE WHEN TG_OP = 'DELETE' THEN OLD.team_id ELSE NEW.team_id END;
    IF TG_OP = 'DELETE' AND NOT EXISTS (SELECT 1 FROM rounds WHERE id = target_round_id) THEN RETURN OLD; END IF;
    IF current_setting('app.score_mutation_round_id', true) IS DISTINCT FROM target_round_id::TEXT THEN
        RAISE EXCEPTION 'scorecard confirmation must use the score workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'score_confirmation_context_required';
    END IF;
    PERFORM acquire_score_round_lock(target_round_id);
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'scorecard confirmations are immutable'
            USING ERRCODE = '23514', CONSTRAINT = 'score_confirmation_immutable';
    END IF;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    SELECT status, scoring_format::TEXT, number_of_holes
    INTO parent_status, parent_format, required_holes FROM rounds WHERE id = target_round_id;
    IF parent_status NOT IN ('open', 'completed')
       AND NOT (parent_status = 'locked' AND current_setting('app.admin_correction', true) = 'true') THEN
        RAISE EXCEPTION 'round is not open for scorecard confirmation'
            USING ERRCODE = '23514', CONSTRAINT = 'score_round_not_editable';
    END IF;
    IF (parent_format = 'individual_stroke_play' AND (target_player_id IS NULL OR target_team_id IS NOT NULL))
       OR (parent_format IN ('team_scramble', 'two_player_foursomes')
           AND (target_team_id IS NULL OR target_player_id IS NOT NULL)) THEN
        RAISE EXCEPTION 'scorecard owner does not match the round format'
            USING ERRCODE = '23514', CONSTRAINT = 'score_owner_format_mismatch';
    END IF;
    IF parent_format = 'two_player_foursomes' AND NOT EXISTS (
        SELECT 1 FROM round_team_handicap_snapshots
        WHERE round_id = target_round_id AND team_id = target_team_id
    ) THEN
        RAISE EXCEPTION 'team was not snapshotted for this round'
            USING ERRCODE = '23514', CONSTRAINT = 'score_owner_ineligible';
    END IF;
    SELECT count(*) INTO scored_holes FROM scores
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

CREATE OR REPLACE FUNCTION round_scorecards_ready(target_round_id UUID) RETURNS BOOLEAN
LANGUAGE plpgsql STABLE AS $$
DECLARE
    target_format TEXT;
    required_holes SMALLINT;
    owner_count BIGINT;
    invalid_owner_count BIGINT;
BEGIN
    SELECT scoring_format::TEXT, number_of_holes INTO target_format, required_holes
    FROM rounds WHERE id = target_round_id;
    IF target_format = 'individual_stroke_play' THEN
        SELECT count(*) INTO owner_count FROM round_handicap_snapshots WHERE round_id = target_round_id;
        SELECT count(*) INTO invalid_owner_count FROM round_handicap_snapshots rhs
        WHERE rhs.round_id = target_round_id
          AND (required_holes <> (SELECT count(*) FROM scores s WHERE s.round_id = target_round_id AND s.player_id = rhs.player_id)
               OR NOT EXISTS (SELECT 1 FROM scorecard_confirmations sc WHERE sc.round_id = target_round_id AND sc.player_id = rhs.player_id));
    ELSIF target_format = 'team_scramble' THEN
        SELECT count(*) INTO owner_count FROM teams WHERE round_id = target_round_id;
        SELECT count(*) INTO invalid_owner_count FROM teams t WHERE t.round_id = target_round_id
          AND (required_holes <> (SELECT count(*) FROM scores s WHERE s.round_id = target_round_id AND s.team_id = t.id)
               OR NOT EXISTS (SELECT 1 FROM scorecard_confirmations sc WHERE sc.round_id = target_round_id AND sc.team_id = t.id));
    ELSIF target_format = 'two_player_foursomes' THEN
        SELECT count(*) INTO owner_count FROM teams WHERE round_id = target_round_id;
        SELECT count(*) INTO invalid_owner_count FROM teams t WHERE t.round_id = target_round_id
          AND (NOT EXISTS (SELECT 1 FROM round_team_handicap_snapshots rths WHERE rths.round_id = t.round_id AND rths.team_id = t.id)
               OR required_holes <> (SELECT count(*) FROM scores s WHERE s.round_id = target_round_id AND s.team_id = t.id)
               OR NOT EXISTS (SELECT 1 FROM scorecard_confirmations sc WHERE sc.round_id = target_round_id AND sc.team_id = t.id));
    ELSE
        RETURN FALSE;
    END IF;
    RETURN owner_count > 0 AND invalid_owner_count = 0;
END;
$$;

CREATE OR REPLACE FUNCTION validate_round_lifecycle_transition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    opening_round_id TEXT;
    required_snapshot_count BIGINT;
    captured_snapshot_count BIGINT;
    required_team_snapshot_count BIGINT;
    captured_team_snapshot_count BIGINT;
BEGIN
    IF NEW.status IS NOT DISTINCT FROM OLD.status THEN RETURN NEW; END IF;
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
        SELECT count(*) INTO required_snapshot_count FROM tournament_players tp
        JOIN players p ON p.id = tp.player_id
        WHERE tp.tournament_id = OLD.tournament_id AND tp.status = 'active' AND p.active;
        SELECT count(*) INTO captured_snapshot_count FROM round_handicap_snapshots WHERE round_id = OLD.id;
        IF captured_snapshot_count <> required_snapshot_count THEN
            RAISE EXCEPTION 'round opening requires one snapshot per active entrant'
                USING ERRCODE = '23514', CONSTRAINT = 'round_opening_snapshots_incomplete';
        END IF;
        IF OLD.scoring_format::TEXT = 'two_player_foursomes' THEN
            SELECT count(*) INTO required_team_snapshot_count FROM teams WHERE round_id = OLD.id;
            SELECT count(*) INTO captured_team_snapshot_count FROM round_team_handicap_snapshots WHERE round_id = OLD.id;
            IF required_team_snapshot_count = 0 OR captured_team_snapshot_count <> required_team_snapshot_count THEN
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
