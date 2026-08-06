ALTER TABLE tees
    ADD CONSTRAINT tees_id_course_id_key UNIQUE (id, course_id);

ALTER TABLE rounds
    ADD CONSTRAINT rounds_course_tee_pair_check
        CHECK ((course_id IS NULL) = (tee_id IS NULL)),
    ADD CONSTRAINT rounds_tee_course_fkey
        FOREIGN KEY (tee_id, course_id) REFERENCES tees(id, course_id) ON DELETE RESTRICT;

-- These cross-checks must not erase participant history when an entrant is
-- deleted directly. Deferring them lets a tournament deletion finish both of
-- its existing cascade paths before referential integrity is checked.
ALTER TABLE round_handicap_snapshots
    DROP CONSTRAINT round_handicap_snapshots_tournament_id_player_id_fkey,
    ADD CONSTRAINT round_handicap_snapshots_tournament_id_player_id_fkey
        FOREIGN KEY (tournament_id, player_id)
        REFERENCES tournament_players(tournament_id, player_id)
        ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE team_memberships
    DROP CONSTRAINT team_memberships_tournament_id_player_id_fkey,
    ADD CONSTRAINT team_memberships_tournament_id_player_id_fkey
        FOREIGN KEY (tournament_id, player_id)
        REFERENCES tournament_players(tournament_id, player_id)
        ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED;

CREATE FUNCTION protect_round_configuration() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.status <> 'draft'
       AND (NEW.tournament_id IS DISTINCT FROM OLD.tournament_id
            OR NEW.round_number IS DISTINCT FROM OLD.round_number
            OR NEW.round_date IS DISTINCT FROM OLD.round_date
            OR NEW.course_id IS DISTINCT FROM OLD.course_id
            OR NEW.tee_id IS DISTINCT FROM OLD.tee_id
            OR NEW.number_of_holes IS DISTINCT FROM OLD.number_of_holes
            OR NEW.handicap_enabled IS DISTINCT FROM OLD.handicap_enabled
            OR NEW.handicap_allowance_percent IS DISTINCT FROM OLD.handicap_allowance_percent
            OR NEW.scoring_format IS DISTINCT FROM OLD.scoring_format) THEN
        RAISE EXCEPTION 'round scoring configuration is frozen after draft'
            USING ERRCODE = '23514', CONSTRAINT = 'round_configuration_frozen';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER rounds_protect_configuration
BEFORE UPDATE ON rounds
FOR EACH ROW EXECUTE FUNCTION protect_round_configuration();

CREATE FUNCTION validate_round_lifecycle_transition() RETURNS trigger LANGUAGE plpgsql AS $$
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
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER rounds_validate_lifecycle
BEFORE UPDATE OF status ON rounds
FOR EACH ROW EXECUTE FUNCTION validate_round_lifecycle_transition();

CREATE FUNCTION protect_round_pairing() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    old_round_id UUID;
    new_round_id UUID;
    parent_status round_status;
BEGIN
    old_round_id = CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE OLD.round_id END;
    new_round_id = CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.round_id END;

    FOR parent_status IN
        SELECT status
        FROM rounds
        WHERE id = old_round_id OR id = new_round_id
        ORDER BY id
        FOR UPDATE
    LOOP
        IF parent_status <> 'draft' THEN
            RAISE EXCEPTION 'round pairings are frozen after draft'
                USING ERRCODE = '23514', CONSTRAINT = 'round_pairing_frozen';
        END IF;
    END LOOP;

    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

CREATE TRIGGER teams_protect_round_pairing
BEFORE INSERT OR UPDATE OR DELETE ON teams
FOR EACH ROW EXECUTE FUNCTION protect_round_pairing();

CREATE TRIGGER team_memberships_protect_round_pairing
BEFORE INSERT OR UPDATE OR DELETE ON team_memberships
FOR EACH ROW EXECUTE FUNCTION protect_round_pairing();

CREATE FUNCTION protect_tee_configuration() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    old_tee_id UUID;
    new_tee_id UUID;
    parent_status round_status;
BEGIN
    old_tee_id = CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE OLD.id END;
    new_tee_id = CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.id END;

    FOR parent_status IN
        SELECT status
        FROM rounds
        WHERE tee_id = old_tee_id OR tee_id = new_tee_id
        ORDER BY id
        FOR UPDATE
    LOOP
        IF parent_status <> 'draft' THEN
            RAISE EXCEPTION 'tee configuration is frozen after round opening'
                USING ERRCODE = '23514', CONSTRAINT = 'tee_configuration_frozen';
        END IF;
    END LOOP;

    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

CREATE TRIGGER tees_protect_configuration
BEFORE UPDATE OR DELETE ON tees
FOR EACH ROW EXECUTE FUNCTION protect_tee_configuration();

CREATE FUNCTION protect_hole_configuration() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    old_tee_id UUID;
    new_tee_id UUID;
    parent_status round_status;
BEGIN
    old_tee_id = CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE OLD.tee_id END;
    new_tee_id = CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.tee_id END;

    FOR parent_status IN
        SELECT status
        FROM rounds
        WHERE tee_id = old_tee_id OR tee_id = new_tee_id
        ORDER BY id
        FOR UPDATE
    LOOP
        IF parent_status <> 'draft' THEN
            RAISE EXCEPTION 'hole configuration is frozen after round opening'
                USING ERRCODE = '23514', CONSTRAINT = 'hole_configuration_frozen';
        END IF;
    END LOOP;

    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

CREATE TRIGGER holes_protect_configuration
BEFORE INSERT OR UPDATE OR DELETE ON holes
FOR EACH ROW EXECUTE FUNCTION protect_hole_configuration();

CREATE FUNCTION protect_round_handicap_snapshot() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_round_id UUID;
    parent_status round_status;
BEGIN
    target_round_id = CASE WHEN TG_OP = 'INSERT' THEN NEW.round_id ELSE OLD.round_id END;
    SELECT status INTO parent_status FROM rounds WHERE id = target_round_id FOR UPDATE;

    IF TG_OP = 'INSERT'
       AND (parent_status IS DISTINCT FROM 'draft'
            OR current_setting('app.round_opening_id', true) IS DISTINCT FROM target_round_id::TEXT) THEN
        RAISE EXCEPTION 'round handicap snapshots can only be captured by the opening workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'round_snapshot_capture_frozen';
    END IF;
    IF TG_OP <> 'INSERT' AND parent_status IS NOT NULL THEN
        RAISE EXCEPTION 'round handicap snapshots are immutable'
            USING ERRCODE = '23514', CONSTRAINT = 'round_snapshot_immutable';
    END IF;

    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

CREATE TRIGGER round_handicap_snapshots_protect
BEFORE INSERT OR UPDATE OR DELETE ON round_handicap_snapshots
FOR EACH ROW EXECUTE FUNCTION protect_round_handicap_snapshot();
