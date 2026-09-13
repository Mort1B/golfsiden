-- Dedicated four-ball inputs; legacy numeric data and receipts remain unchanged.
ALTER TYPE scoring_format ADD VALUE 'four_ball_stroke_play';
ALTER TABLE rounds ADD CONSTRAINT four_ball_18_holes CHECK(scoring_format::text <> 'four_ball_stroke_play' OR number_of_holes=18);
CREATE TABLE four_ball_inputs (
 id UUID PRIMARY KEY,
 round_id UUID NOT NULL,
 tournament_id UUID NOT NULL,
 hole_id UUID NOT NULL REFERENCES holes(id) ON DELETE RESTRICT,
 player_id UUID NOT NULL,
 gross_strokes SMALLINT CHECK(gross_strokes BETWEEN 1 AND 20),
 revision BIGINT NOT NULL DEFAULT 1 CHECK(revision>0),
 submitted_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
 submitted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(round_id,hole_id,player_id),
 FOREIGN KEY(round_id,tournament_id) REFERENCES rounds(id,tournament_id) ON DELETE CASCADE,
 FOREIGN KEY(round_id,player_id) REFERENCES round_handicap_snapshots(round_id,player_id) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED
);
-- Side/card reads select both players in one round.
CREATE INDEX four_ball_inputs_player_idx ON four_ball_inputs(round_id,player_id,hole_id);
CREATE TABLE four_ball_input_audits (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 input_id UUID NOT NULL REFERENCES four_ball_inputs(id) ON DELETE CASCADE,
 revision BIGINT NOT NULL CHECK(revision>0),
 previous_gross_strokes SMALLINT CHECK(previous_gross_strokes BETWEEN 1 AND 20),
 gross_strokes SMALLINT CHECK(gross_strokes BETWEEN 1 AND 20),
 changed_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
 changed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(input_id,revision)
);
CREATE FUNCTION four_ball_player_authorized(target_round UUID,target_player UUID,actor UUID,session UUID) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM user_sessions s JOIN users u ON u.id=s.user_id
 JOIN rounds r ON r.id=target_round JOIN tournament_memberships m ON m.tournament_id=r.tournament_id AND m.user_id=u.id
 WHERE s.id=session AND u.id=actor AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND s.credential_generation=u.credential_generation
 AND (m.role IN ('admin','scorer') OR (m.role='player' AND (u.player_id=target_player OR EXISTS(
 SELECT 1 FROM flight_memberships a JOIN flight_memberships b ON b.round_id=a.round_id AND b.flight_id=a.flight_id
 WHERE a.round_id=target_round AND a.player_id=u.player_id AND b.player_id=target_player)))))
$$;
CREATE FUNCTION guard_four_ball_input() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r rounds%ROWTYPE; actor UUID; session UUID;
BEGIN
 IF TG_OP='DELETE' AND NOT EXISTS(SELECT 1 FROM rounds WHERE id=OLD.round_id) THEN RETURN OLD; END IF;
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'four-ball inputs retain their identity' USING ERRCODE='23514'; END IF;
 PERFORM acquire_score_round_lock(NEW.round_id);
 SELECT * INTO r FROM rounds WHERE id=NEW.round_id;
 actor=NULLIF(current_setting('app.four_ball_actor_id',true),'')::UUID;
 session=NULLIF(current_setting('app.four_ball_session_id',true),'')::UUID;
 IF r.scoring_format::text IS DISTINCT FROM 'four_ball_stroke_play' OR r.status NOT IN ('open','completed')
 OR current_setting('app.score_mutation_round_id',true) IS DISTINCT FROM NEW.round_id::text
 OR actor IS DISTINCT FROM NEW.submitted_by OR NOT four_ball_player_authorized(NEW.round_id,NEW.player_id,actor,session)
 OR NOT EXISTS(SELECT 1 FROM holes WHERE id=NEW.hole_id AND tee_id=r.tee_id)
 OR NOT EXISTS(SELECT 1 FROM team_memberships WHERE round_id=NEW.round_id AND player_id=NEW.player_id) THEN
 RAISE EXCEPTION 'invalid four-ball input context' USING ERRCODE='23514',CONSTRAINT='four_ball_input_context'; END IF;
 IF TG_OP='INSERT' THEN
 IF NEW.revision<>1 THEN RAISE EXCEPTION 'input revision is managed' USING ERRCODE='23514'; END IF;
 ELSE
 IF (NEW.id,NEW.round_id,NEW.tournament_id,NEW.hole_id,NEW.player_id,NEW.submitted_at,NEW.revision) IS DISTINCT FROM (OLD.id,OLD.round_id,OLD.tournament_id,OLD.hole_id,OLD.player_id,OLD.submitted_at,OLD.revision) THEN
 RAISE EXCEPTION 'input identity and revision are managed' USING ERRCODE='23514'; END IF;
 IF NEW.gross_strokes IS NOT DISTINCT FROM OLD.gross_strokes THEN
 IF NEW.submitted_by IS DISTINCT FROM OLD.submitted_by THEN RAISE EXCEPTION 'unchanged input actor is immutable' USING ERRCODE='23514'; END IF;
 NEW.updated_at=OLD.updated_at;
 ELSE
 IF OLD.revision=9223372036854775807 THEN RAISE EXCEPTION 'input revision exhausted' USING ERRCODE='23514'; END IF;
 NEW.revision=OLD.revision+1;NEW.updated_at=clock_timestamp();
 END IF;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER four_ball_inputs_guard BEFORE INSERT OR UPDATE OR DELETE ON four_ball_inputs FOR EACH ROW EXECUTE FUNCTION guard_four_ball_input();
CREATE FUNCTION audit_four_ball_input() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' OR NEW.gross_strokes IS DISTINCT FROM OLD.gross_strokes THEN
 INSERT INTO four_ball_input_audits(input_id,revision,previous_gross_strokes,gross_strokes,changed_by)
 VALUES(NEW.id,NEW.revision,CASE WHEN TG_OP='UPDATE' THEN OLD.gross_strokes ELSE NULL END,NEW.gross_strokes,NEW.submitted_by);
 DELETE FROM scorecard_confirmations WHERE round_id=NEW.round_id AND team_id IN(SELECT team_id FROM team_memberships WHERE round_id=NEW.round_id AND player_id=NEW.player_id);
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER four_ball_inputs_audit AFTER INSERT OR UPDATE ON four_ball_inputs FOR EACH ROW EXECUTE FUNCTION audit_four_ball_input();
CREATE FUNCTION guard_four_ball_audit() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' AND pg_trigger_depth()>1 AND EXISTS(SELECT 1 FROM four_ball_inputs i WHERE i.id=NEW.input_id AND i.revision=NEW.revision AND i.submitted_by=NEW.changed_by AND i.gross_strokes IS NOT DISTINCT FROM NEW.gross_strokes) THEN RETURN NEW; END IF;
 IF TG_OP='DELETE' AND NOT EXISTS(SELECT 1 FROM four_ball_inputs WHERE id=OLD.input_id) THEN RETURN OLD; END IF;
 RAISE EXCEPTION 'four-ball input audits are append-only' USING ERRCODE='23514';
END $$;
CREATE TRIGGER four_ball_audits_guard BEFORE INSERT OR UPDATE OR DELETE ON four_ball_input_audits FOR EACH ROW EXECUTE FUNCTION guard_four_ball_audit();
CREATE FUNCTION four_ball_side_holes(target_round UUID,target_team UUID) RETURNS BIGINT LANGUAGE sql STABLE AS $$
 SELECT count(DISTINCT i.hole_id) FROM four_ball_inputs i JOIN team_memberships m ON m.round_id=i.round_id AND m.player_id=i.player_id WHERE i.round_id=target_round AND m.team_id=target_team AND i.gross_strokes IS NOT NULL
$$;
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
    IF parent_format = 'four_ball_stroke_play' THEN RAISE EXCEPTION 'four-ball uses dedicated inputs' USING ERRCODE='23514',CONSTRAINT='score_owner_format_mismatch'; END IF;
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
       OR (parent_format IN ('team_scramble', 'two_player_foursomes','four_ball_stroke_play')
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
    IF parent_format='four_ball_stroke_play' THEN
      IF parent_status NOT IN ('open','completed') OR EXISTS(
        SELECT 1 FROM team_memberships m WHERE m.round_id=target_round_id AND m.team_id=target_team_id AND NOT four_ball_player_authorized(target_round_id,m.player_id,NEW.confirmed_by,NULLIF(current_setting('app.four_ball_session_id',true),'')::UUID)
      ) OR (SELECT count(*) FROM team_memberships WHERE round_id=target_round_id AND team_id=target_team_id)<>2 THEN
        RAISE EXCEPTION 'four-ball confirmation requires both player cards' USING ERRCODE='23514'; END IF;
      scored_holes=four_ball_side_holes(target_round_id,target_team_id);
    ELSE
    SELECT count(*) INTO scored_holes FROM scores
    WHERE round_id = target_round_id
      AND ((target_player_id IS NOT NULL AND player_id = target_player_id)
           OR (target_team_id IS NOT NULL AND team_id = target_team_id));
    END IF;
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
    ELSIF target_format = 'four_ball_stroke_play' THEN
        SELECT count(*) INTO owner_count FROM teams WHERE round_id=target_round_id;
        SELECT count(*) INTO invalid_owner_count FROM teams t WHERE t.round_id=target_round_id AND
          (four_ball_side_holes(target_round_id,t.id)<>required_holes OR NOT EXISTS(SELECT 1 FROM scorecard_confirmations c WHERE c.round_id=target_round_id AND c.team_id=t.id));
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
        IF OLD.scoring_format::TEXT='four_ball_stroke_play' THEN
            IF NEW.number_of_holes<>18 OR required_snapshot_count=0
            OR EXISTS(SELECT 1 FROM round_handicap_snapshots s WHERE s.round_id=OLD.id AND NOT EXISTS(SELECT 1 FROM team_memberships m WHERE m.round_id=s.round_id AND m.player_id=s.player_id))
            OR NOT EXISTS(SELECT 1 FROM teams WHERE round_id=OLD.id)
            OR EXISTS(SELECT 1 FROM teams t LEFT JOIN team_memberships m ON m.team_id=t.id AND m.round_id=t.round_id LEFT JOIN round_handicap_snapshots s ON s.round_id=m.round_id AND s.player_id=m.player_id LEFT JOIN flight_memberships f ON f.round_id=m.round_id AND f.player_id=m.player_id WHERE t.round_id=OLD.id GROUP BY t.id HAVING count(m.player_id)<>2 OR count(s.player_id)<>2 OR count(f.player_id)<>2 OR count(DISTINCT f.flight_id)<>1)
            THEN RAISE EXCEPTION 'four-ball opening requires exact eligible sides in one flight and 18 holes' USING ERRCODE='23514'; END IF;
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
CREATE TABLE four_ball_mutation_receipts (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id UUID NOT NULL,
    round_id UUID NOT NULL REFERENCES rounds(id) ON DELETE CASCADE,
    request_hash BYTEA NOT NULL CHECK(octet_length(request_hash)=32),
    applied_score_id UUID NOT NULL REFERENCES four_ball_inputs(id) ON DELETE CASCADE,
    applied_revision BIGINT NOT NULL CHECK(applied_revision>0),
    applied_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(user_id,request_id)
);
CREATE INDEX four_ball_mutation_receipts_round_idx ON four_ball_mutation_receipts(round_id);
CREATE INDEX four_ball_mutation_receipts_score_idx ON four_ball_mutation_receipts(applied_score_id);
CREATE FUNCTION guard_four_ball_mutation_receipt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='INSERT' THEN
        IF current_setting('app.four_ball_actor_id',true) IS DISTINCT FROM NEW.user_id::TEXT
           OR current_setting('app.four_ball_request_id',true) IS DISTINCT FROM NEW.request_id::TEXT
           OR current_setting('app.score_mutation_round_id',true) IS DISTINCT FROM NEW.round_id::TEXT
           OR NOT EXISTS(SELECT 1 FROM four_ball_inputs WHERE id=NEW.applied_score_id AND round_id=NEW.round_id AND revision=NEW.applied_revision) THEN
            RAISE EXCEPTION 'score receipts require the conditional scoring workflow' USING ERRCODE='23514',CONSTRAINT='score_receipt_context_required';
        END IF;
        RETURN NEW;
    END IF;
    IF TG_OP='UPDATE' OR (
        EXISTS(SELECT 1 FROM users WHERE id=OLD.user_id)
        AND EXISTS(SELECT 1 FROM rounds WHERE id=OLD.round_id)
        AND EXISTS(SELECT 1 FROM four_ball_inputs WHERE id=OLD.applied_score_id)
    ) THEN
        RAISE EXCEPTION 'score receipts are immutable' USING ERRCODE='23514',CONSTRAINT='score_receipt_immutable';
    END IF;
    RETURN OLD;
END;
$$;
CREATE TRIGGER four_ball_mutation_receipts_guard BEFORE INSERT OR UPDATE OR DELETE ON four_ball_mutation_receipts
FOR EACH ROW EXECUTE FUNCTION guard_four_ball_mutation_receipt();