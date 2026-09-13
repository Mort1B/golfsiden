-- Dedicated singles aggregates. Published stroke/Stableford data is unchanged.
ALTER TYPE scoring_format ADD VALUE 'singles_match_play';
ALTER TABLE rounds ADD CONSTRAINT singles_match_settings CHECK(scoring_format::text <> 'singles_match_play' OR (number_of_holes=18 AND handicap_allowance_percent=100));
CREATE TABLE singles_matches (
 id UUID PRIMARY KEY,
 round_id UUID NOT NULL,
 tournament_id UUID NOT NULL,
 first_player_id UUID NOT NULL,
 second_player_id UUID NOT NULL,
 revision BIGINT NOT NULL DEFAULT 1 CHECK(revision>0),
 ledger JSONB NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(ledger)='array' AND jsonb_array_length(ledger)<=19),
 terminal BOOLEAN NOT NULL DEFAULT FALSE,
 confirmed BOOLEAN NOT NULL DEFAULT FALSE,
 correction_pending BOOLEAN NOT NULL DEFAULT FALSE,
 first_half_points SMALLINT,
 second_half_points SMALLINT,
 CHECK(first_player_id<>second_player_id),
 CHECK((NOT confirmed AND first_half_points IS NULL AND second_half_points IS NULL) OR (confirmed AND terminal AND first_half_points IS NOT NULL AND second_half_points IS NOT NULL AND first_half_points BETWEEN 0 AND 2 AND second_half_points=2-first_half_points)),
 UNIQUE(id,round_id,tournament_id),
 FOREIGN KEY(round_id,tournament_id) REFERENCES rounds(id,tournament_id) ON DELETE CASCADE,
 FOREIGN KEY(tournament_id,first_player_id) REFERENCES tournament_players(tournament_id,player_id) ON DELETE RESTRICT,
 FOREIGN KEY(tournament_id,second_player_id) REFERENCES tournament_players(tournament_id,player_id) ON DELETE RESTRICT
);
CREATE INDEX singles_matches_round_idx ON singles_matches(round_id,id);
CREATE TABLE singles_match_opponents (
 match_id UUID NOT NULL,
 round_id UUID NOT NULL,
 tournament_id UUID NOT NULL,
 player_id UUID NOT NULL,
 PRIMARY KEY(match_id,player_id),
 UNIQUE(round_id,player_id),
 FOREIGN KEY(match_id,round_id,tournament_id) REFERENCES singles_matches(id,round_id,tournament_id) ON DELETE CASCADE,
 FOREIGN KEY(tournament_id,player_id) REFERENCES tournament_players(tournament_id,player_id) ON DELETE RESTRICT
);
CREATE TABLE singles_match_notes (
 match_id UUID NOT NULL,
 player_id UUID NOT NULL,
 hole_number SMALLINT NOT NULL CHECK(hole_number BETWEEN 1 AND 18),
 gross_strokes SMALLINT CHECK(gross_strokes BETWEEN 1 AND 20),
 changed_revision BIGINT NOT NULL CHECK(changed_revision>1),
 PRIMARY KEY(match_id,player_id,hole_number),
 FOREIGN KEY(match_id,player_id) REFERENCES singles_match_opponents(match_id,player_id) ON DELETE CASCADE
);
CREATE TABLE singles_match_audits (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 match_id UUID NOT NULL REFERENCES singles_matches(id) ON DELETE CASCADE,
 revision BIGINT NOT NULL CHECK(revision>1),
 actor_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
 request_id UUID NOT NULL,
 command JSONB NOT NULL,
 previous_ledger JSONB NOT NULL,
 ledger JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(match_id,revision)
);
CREATE TABLE singles_match_receipts (
 user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 request_id UUID NOT NULL,
 match_id UUID NOT NULL REFERENCES singles_matches(id) ON DELETE CASCADE,
 request_hash BYTEA NOT NULL CHECK(octet_length(request_hash)=32),
 applied_revision BIGINT NOT NULL CHECK(applied_revision>0),
 PRIMARY KEY(user_id,request_id)
);
CREATE INDEX singles_match_receipts_match_idx ON singles_match_receipts(match_id);
CREATE FUNCTION singles_admin(target_round UUID,actor UUID,session UUID) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT stableford_player_authorized(target_round,NULL,actor,session) AND EXISTS(SELECT 1 FROM tournament_memberships m JOIN rounds r ON r.tournament_id=m.tournament_id WHERE r.id=target_round AND m.user_id=actor AND m.role='admin')
$$;
CREATE FUNCTION guard_singles_match() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r rounds%ROWTYPE; actor UUID; session UUID; command JSONB; kind TEXT;
BEGIN
 IF TG_OP='DELETE' AND NOT EXISTS(SELECT 1 FROM rounds WHERE id=OLD.round_id) THEN RETURN OLD; END IF;
 IF TG_OP='INSERT' THEN PERFORM acquire_score_round_lock(NEW.round_id); SELECT * INTO r FROM rounds WHERE id=NEW.round_id;
 ELSE PERFORM acquire_score_round_lock(OLD.round_id); SELECT * INTO r FROM rounds WHERE id=OLD.round_id; END IF;
 actor=NULLIF(current_setting('app.match_actor',true),'')::UUID;
 session=NULLIF(current_setting('app.match_session',true),'')::UUID;
 IF r.scoring_format::text IS DISTINCT FROM 'singles_match_play' THEN RAISE EXCEPTION 'match format required' USING ERRCODE='23514'; END IF;
 IF TG_OP IN ('INSERT','DELETE') THEN
   IF r.status<>'draft' OR NOT singles_admin(r.id,actor,session) THEN RAISE EXCEPTION 'match assignments require draft admin' USING ERRCODE='23514'; END IF;
   IF TG_OP='DELETE' THEN RETURN OLD; END IF;
   IF NEW.revision<>1 OR NEW.ledger<>'[]'::jsonb OR NEW.terminal OR NEW.confirmed OR NEW.correction_pending THEN RAISE EXCEPTION 'new match must be unstarted' USING ERRCODE='23514'; END IF;
   RETURN NEW;
 END IF;
 IF (NEW.id,NEW.round_id,NEW.tournament_id,NEW.first_player_id,NEW.second_player_id) IS DISTINCT FROM (OLD.id,OLD.round_id,OLD.tournament_id,OLD.first_player_id,OLD.second_player_id) THEN RAISE EXCEPTION 'match identity is immutable' USING ERRCODE='23514'; END IF;
 IF NEW.revision<>OLD.revision+1 OR NOT stableford_player_authorized(r.id,OLD.first_player_id,actor,session) OR NOT stableford_player_authorized(r.id,OLD.second_player_id,actor,session) THEN RAISE EXCEPTION 'match revision and authority required' USING ERRCODE='23514'; END IF;
 command=NULLIF(current_setting('app.match_command',true),'')::JSONB;kind=command->>'type';
 IF kind IS NULL OR NULLIF(current_setting('app.match_request',true),'') IS NULL THEN RAISE EXCEPTION 'match command context required' USING ERRCODE='23514'; END IF;
 IF r.status='draft' OR (r.status='locked' AND NOT (singles_admin(r.id,actor,session) AND (kind='correct' OR (kind='confirm' AND OLD.correction_pending)))) THEN RAISE EXCEPTION 'match lifecycle rejects command' USING ERRCODE='23514'; END IF;
 IF OLD.terminal AND kind NOT IN ('correct','confirm') THEN RAISE EXCEPTION 'terminal match requires correction' USING ERRCODE='23514'; END IF;
 IF kind='correct' AND (NOT singles_admin(r.id,actor,session) OR COALESCE(length(btrim(command->>'reason')),0) NOT BETWEEN 1 AND 1000 OR NEW.confirmed OR NOT NEW.correction_pending) THEN RAISE EXCEPTION 'correction requires reason and invalidation' USING ERRCODE='23514'; END IF;
 IF kind IN ('note','clear_note','confirm') AND NEW.ledger IS DISTINCT FROM OLD.ledger THEN RAISE EXCEPTION 'command cannot rewrite accepted ledger' USING ERRCODE='23514'; END IF;
 IF kind='report' AND (jsonb_array_length(NEW.ledger)<>jsonb_array_length(OLD.ledger)+1 OR NEW.ledger - (jsonb_array_length(NEW.ledger)-1) <> OLD.ledger) THEN RAISE EXCEPTION 'reports must append' USING ERRCODE='23514'; END IF;
 IF NEW.confirmed AND (kind<>'confirm' OR NOT NEW.terminal OR (command->>'result_agreed_or_awarded') IS DISTINCT FROM 'true') THEN RAISE EXCEPTION 'confirmation requires terminal attestation' USING ERRCODE='23514'; END IF;
 IF kind NOT IN ('note','clear_note','report','correct','confirm') THEN RAISE EXCEPTION 'unknown match command' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER singles_match_guard BEFORE INSERT OR UPDATE OR DELETE ON singles_matches FOR EACH ROW EXECUTE FUNCTION guard_singles_match();
CREATE FUNCTION maintain_singles_match() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
 INSERT INTO singles_match_opponents(match_id,round_id,tournament_id,player_id) VALUES(NEW.id,NEW.round_id,NEW.tournament_id,NEW.first_player_id),(NEW.id,NEW.round_id,NEW.tournament_id,NEW.second_player_id);
 ELSE
 INSERT INTO singles_match_audits(match_id,revision,actor_id,request_id,command,previous_ledger,ledger) VALUES(NEW.id,NEW.revision,current_setting('app.match_actor')::UUID,current_setting('app.match_request')::UUID,current_setting('app.match_command')::JSONB,OLD.ledger,NEW.ledger);
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER singles_match_maintain AFTER INSERT OR UPDATE ON singles_matches FOR EACH ROW EXECUTE FUNCTION maintain_singles_match();
CREATE FUNCTION guard_singles_child() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE m singles_matches%ROWTYPE;
BEGIN
 IF TG_OP='DELETE' AND NOT EXISTS(SELECT 1 FROM singles_matches WHERE id=OLD.match_id) THEN RETURN OLD; END IF;
 IF TG_TABLE_NAME='singles_match_opponents' OR TG_TABLE_NAME='singles_match_audits' THEN
 IF TG_OP='INSERT' AND pg_trigger_depth()>1 THEN RETURN NEW; END IF;
 RAISE EXCEPTION 'match opponents and audit are managed and immutable' USING ERRCODE='23514';
 END IF;
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'match notes retain identity' USING ERRCODE='23514'; END IF;
 SELECT * INTO m FROM singles_matches WHERE id=NEW.match_id;
 PERFORM acquire_score_round_lock(m.round_id);
 SELECT * INTO m FROM singles_matches WHERE id=NEW.match_id FOR UPDATE;
 IF current_setting('app.match_note_id',true) IS DISTINCT FROM m.id::text
 OR current_setting('app.match_actor',true) IS NULL
 OR NOT stableford_player_authorized(m.round_id,NEW.player_id,NULLIF(current_setting('app.match_actor',true),'')::UUID,NULLIF(current_setting('app.match_session',true),'')::UUID)
 OR m.terminal OR (SELECT status FROM rounds WHERE id=m.round_id) NOT IN ('open','completed') THEN RAISE EXCEPTION 'numeric note context required' USING ERRCODE='23514'; END IF;
 NEW.changed_revision=m.revision+1;
 IF TG_OP='UPDATE' AND (NEW.match_id,NEW.player_id,NEW.hole_number) IS DISTINCT FROM (OLD.match_id,OLD.player_id,OLD.hole_number) THEN RAISE EXCEPTION 'note identity is immutable' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER singles_opponents_guard BEFORE INSERT OR UPDATE OR DELETE ON singles_match_opponents FOR EACH ROW EXECUTE FUNCTION guard_singles_child();
CREATE TRIGGER singles_audits_guard BEFORE INSERT OR UPDATE OR DELETE ON singles_match_audits FOR EACH ROW EXECUTE FUNCTION guard_singles_child();
CREATE TRIGGER singles_notes_guard BEFORE INSERT OR UPDATE OR DELETE ON singles_match_notes FOR EACH ROW EXECUTE FUNCTION guard_singles_child();
CREATE FUNCTION guard_singles_receipt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' AND current_setting('app.match_actor',true)=NEW.user_id::text AND current_setting('app.match_request',true)=NEW.request_id::text AND EXISTS(SELECT 1 FROM singles_matches WHERE id=NEW.match_id AND revision=NEW.applied_revision) THEN RETURN NEW; END IF;
 IF TG_OP='DELETE' AND (NOT EXISTS(SELECT 1 FROM singles_matches WHERE id=OLD.match_id) OR NOT EXISTS(SELECT 1 FROM users WHERE id=OLD.user_id)) THEN RETURN OLD; END IF;
 RAISE EXCEPTION 'match receipts are managed and immutable' USING ERRCODE='23514';
END $$;
CREATE TRIGGER singles_receipt_guard BEFORE INSERT OR UPDATE OR DELETE ON singles_match_receipts FOR EACH ROW EXECUTE FUNCTION guard_singles_receipt();
CREATE FUNCTION singles_assignments_ready(target UUID) RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM singles_matches WHERE round_id=target)
 AND NOT EXISTS(SELECT 1 FROM tournament_players tp JOIN rounds r ON r.tournament_id=tp.tournament_id JOIN players p ON p.id=tp.player_id WHERE r.id=target AND tp.status='active' AND p.active AND NOT EXISTS(SELECT 1 FROM singles_match_opponents o WHERE o.round_id=target AND o.player_id=tp.player_id))
 AND NOT EXISTS(SELECT 1 FROM singles_match_opponents o JOIN tournament_players tp ON tp.tournament_id=o.tournament_id AND tp.player_id=o.player_id JOIN players p ON p.id=o.player_id WHERE o.round_id=target AND (tp.status<>'active' OR NOT p.active))
 AND NOT EXISTS(SELECT 1 FROM singles_matches m LEFT JOIN flight_memberships a ON a.round_id=m.round_id AND a.player_id=m.first_player_id LEFT JOIN flight_memberships b ON b.round_id=m.round_id AND b.player_id=m.second_player_id WHERE m.round_id=target AND (a.flight_id IS NULL OR b.flight_id IS NULL OR a.flight_id<>b.flight_id))
 AND NOT EXISTS(SELECT 1 FROM flights WHERE round_id=target AND COALESCE(starting_hole,1)<>1)
$$;
CREATE FUNCTION guard_singles_round() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.scoring_format::text='singles_match_play' AND OLD.status='draft' AND NEW.status='open' THEN
 IF NOT singles_assignments_ready(NEW.id) OR EXISTS(SELECT 1 FROM teams WHERE round_id=NEW.id) THEN RAISE EXCEPTION 'singles opening requires complete same-flight opponents starting at hole one' USING ERRCODE='23514'; END IF;
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER singles_round_guard BEFORE UPDATE OF status ON rounds FOR EACH ROW EXECUTE FUNCTION guard_singles_round();
ALTER FUNCTION round_scorecards_ready(UUID) RENAME TO pre_singles_round_scorecards_ready;
CREATE FUNCTION round_scorecards_ready(target_round_id UUID) RETURNS BOOLEAN LANGUAGE plpgsql STABLE AS $$
BEGIN
 IF (SELECT scoring_format::text FROM rounds WHERE id=target_round_id)='singles_match_play' THEN
 RETURN EXISTS(SELECT 1 FROM singles_matches WHERE round_id=target_round_id) AND NOT EXISTS(SELECT 1 FROM singles_matches WHERE round_id=target_round_id AND (NOT terminal OR NOT confirmed));
 END IF;RETURN pre_singles_round_scorecards_ready(target_round_id);
END $$;
CREATE FUNCTION reject_singles_legacy_score() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (SELECT scoring_format::text FROM rounds WHERE id=NEW.round_id)='singles_match_play' THEN RAISE EXCEPTION 'singles uses its match aggregate' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER singles_legacy_score_guard BEFORE INSERT OR UPDATE ON scores FOR EACH ROW EXECUTE FUNCTION reject_singles_legacy_score();
CREATE TRIGGER singles_legacy_confirmation_guard BEFORE INSERT OR UPDATE ON scorecard_confirmations FOR EACH ROW EXECUTE FUNCTION reject_singles_legacy_score();

-- A note can become visible only with its aggregate revision, audit and receipt.
CREATE FUNCTION check_singles_note_command() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM singles_match_audits a JOIN singles_match_receipts r ON r.match_id=a.match_id AND r.user_id=a.actor_id AND r.request_id=a.request_id AND r.applied_revision=a.revision
 WHERE a.match_id=NEW.match_id AND a.revision=NEW.changed_revision
 AND (a.command->>'player_id')=NEW.player_id::text AND (a.command->>'hole_number')=NEW.hole_number::text
 AND ((a.command->>'type'='note' AND (a.command->>'gross_strokes')=NEW.gross_strokes::text)
 OR (a.command->>'type'='clear_note' AND NEW.gross_strokes IS NULL)))
 THEN RAISE EXCEPTION 'note requires matching committed aggregate command' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER singles_note_command_check AFTER INSERT OR UPDATE ON singles_match_notes DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_singles_note_command();
