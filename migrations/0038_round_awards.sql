-- Personal net awards are independent of hole scores and Fantasy source facts.
CREATE TYPE round_award_type AS ENUM ('closest_to_pin', 'longest_drive');
CREATE TABLE round_award_history (
    request_id UUID PRIMARY KEY,
    round_id UUID NOT NULL REFERENCES rounds(id) ON DELETE RESTRICT,
    award_type round_award_type NOT NULL,
    player_id UUID,
    previous_player_id UUID,
    revision BIGINT NOT NULL CHECK (revision > 0),
    reason TEXT NOT NULL CHECK (btrim(reason) <> '' AND octet_length(reason) <= 500),
    correction BOOLEAN NOT NULL,
    actor_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    command JSONB NOT NULL,
    UNIQUE(round_id, award_type, revision),
    FOREIGN KEY(round_id, player_id) REFERENCES round_handicap_snapshots(round_id, player_id) ON DELETE RESTRICT,
    FOREIGN KEY(round_id, previous_player_id) REFERENCES round_handicap_snapshots(round_id, player_id) ON DELETE RESTRICT
);
CREATE TABLE round_awards (
    round_id UUID NOT NULL REFERENCES rounds(id) ON DELETE RESTRICT,
    award_type round_award_type NOT NULL,
    player_id UUID,
    revision BIGINT NOT NULL CHECK (revision > 0),
    PRIMARY KEY(round_id, award_type),
    FOREIGN KEY(round_id, player_id) REFERENCES round_handicap_snapshots(round_id, player_id) ON DELETE RESTRICT,
    FOREIGN KEY(round_id, award_type, revision) REFERENCES round_award_history(round_id, award_type, revision) ON DELETE RESTRICT
);
-- The history index supports the bounded round admin history read.
CREATE INDEX round_award_history_round_order ON round_award_history(round_id, created_at, request_id);
CREATE FUNCTION guard_round_award_history() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r rounds%ROWTYPE; actor UUID; previous round_awards%ROWTYPE;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'round award history is immutable' USING ERRCODE='23514', CONSTRAINT='round_award_history_immutable';
    END IF;
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'round awards require read committed' USING ERRCODE='23514', CONSTRAINT='round_award_isolation';
    END IF;
    SELECT * INTO r FROM rounds WHERE id=NEW.round_id FOR UPDATE;
    IF r.status NOT IN ('completed','locked') OR r.scoring_format='singles_match_play' THEN
        RAISE EXCEPTION 'unsupported round award state' USING ERRCODE='23514', CONSTRAINT='round_award_state';
    END IF;
    SELECT s.user_id INTO actor FROM user_sessions s JOIN users u ON u.id=s.user_id
    WHERE s.id::text=current_setting('app.round_award_session_id',true)
      AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
      AND s.credential_generation=u.credential_generation FOR SHARE OF s,u;
    IF actor IS NULL OR actor IS DISTINCT FROM NEW.actor_id THEN
        RAISE EXCEPTION 'round award requires active actor' USING ERRCODE='23514', CONSTRAINT='round_award_session';
    END IF;
    PERFORM 1 FROM tournament_memberships WHERE tournament_id=r.tournament_id AND user_id=actor AND role='admin' FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'round award requires exact admin' USING ERRCODE='23514', CONSTRAINT='round_award_admin';
    END IF;
    SELECT * INTO previous FROM round_awards WHERE round_id=NEW.round_id AND award_type=NEW.award_type;
    IF NEW.revision <> coalesce(previous.revision,0)+1 OR NEW.previous_player_id IS DISTINCT FROM previous.player_id
       OR (NEW.player_id IS NULL AND previous.player_id IS NULL) THEN
        RAISE EXCEPTION 'round award revision mismatch' USING ERRCODE='23514', CONSTRAINT='round_award_revision';
    END IF;
    IF (r.status='locked' OR previous.revision IS NOT NULL OR NEW.player_id IS NULL) AND NOT NEW.correction THEN
        RAISE EXCEPTION 'round award requires correction acknowledgement' USING ERRCODE='23514', CONSTRAINT='round_award_correction';
    END IF;
    IF NEW.command IS DISTINCT FROM jsonb_build_object(
        'version',1,'request_id',NEW.request_id,'award_type',NEW.award_type,'player_id',NEW.player_id,
        'expected_revision',(NEW.revision-1)::text,'reason',NEW.command->>'reason','correction',NEW.correction)
        OR (NEW.command->>'reason') IS DISTINCT FROM NEW.reason THEN
        RAISE EXCEPTION 'round award receipt must match command' USING ERRCODE='23514', CONSTRAINT='round_award_command';
    END IF;
    NEW.created_at := clock_timestamp();
    RETURN NEW;
END $$;
CREATE TRIGGER round_award_history_guard BEFORE INSERT OR UPDATE OR DELETE ON round_award_history
FOR EACH ROW EXECUTE FUNCTION guard_round_award_history();
CREATE FUNCTION guard_round_award_current() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' THEN
        RAISE EXCEPTION 'remove round awards through audited null winner' USING ERRCODE='23514', CONSTRAINT='round_award_current_delete';
    END IF;
    PERFORM 1 FROM rounds WHERE id=NEW.round_id FOR UPDATE;
    IF (TG_OP='UPDATE' AND (NEW.round_id IS DISTINCT FROM OLD.round_id OR NEW.award_type IS DISTINCT FROM OLD.award_type OR NEW.revision<>OLD.revision+1))
       OR (TG_OP='INSERT' AND NEW.revision<>1)
       OR NOT EXISTS (SELECT 1 FROM round_award_history h WHERE h.round_id=NEW.round_id AND h.award_type=NEW.award_type AND h.revision=NEW.revision AND h.player_id IS NOT DISTINCT FROM NEW.player_id) THEN
        RAISE EXCEPTION 'round award must match next audited command' USING ERRCODE='23514', CONSTRAINT='round_award_current_history';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER round_award_current_guard BEFORE INSERT OR UPDATE OR DELETE ON round_awards
FOR EACH ROW EXECUTE FUNCTION guard_round_award_current();
-- An accepted audit command must be applied atomically to its current slot.
CREATE FUNCTION check_round_award_applied() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM round_awards a WHERE a.round_id=NEW.round_id AND a.award_type=NEW.award_type AND a.revision>=NEW.revision) THEN
        RAISE EXCEPTION 'round award audit was not applied' USING ERRCODE='23514', CONSTRAINT='round_award_applied';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER round_award_applied AFTER INSERT ON round_award_history DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_round_award_applied();
