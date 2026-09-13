-- Structural validation of the accepted sequence backs the domain derivation.
-- This function never derives a report from numeric notes or calculates handicaps.
CREATE FUNCTION singles_ledger_result(ledger JSONB,first_player UUID,second_player UUID)
RETURNS TABLE(terminal BOOLEAN,first_points SMALLINT) LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE item JSONB; e JSONB; n INTEGER=0; lead INTEGER=0; winner TEXT; kind TEXT; outcome TEXT;
BEGIN
 terminal=FALSE;first_points=NULL;
 FOR item IN SELECT value FROM jsonb_array_elements(ledger) LOOP
 IF terminal OR jsonb_typeof(item)<>'object' OR item->>'id' IS NULL OR jsonb_typeof(item->'event') IS DISTINCT FROM 'object' THEN RAISE EXCEPTION 'invalid accepted event' USING ERRCODE='23514'; END IF;
 e=item->'event';kind=e->>'type';
 IF kind='hole' THEN
 IF (e->>'hole_number')::int IS DISTINCT FROM n+1 OR n>=18 THEN RAISE EXCEPTION 'noncontiguous match report' USING ERRCODE='23514'; END IF;
 n=n+1;outcome=e->>'outcome';
 IF outcome NOT IN ('first','second','halved') OR outcome IS NULL THEN RAISE EXCEPTION 'invalid hole outcome' USING ERRCODE='23514'; END IF;
 lead=lead+CASE outcome WHEN 'first' THEN 1 WHEN 'second' THEN -1 ELSE 0 END;
 IF abs(lead)>18-n THEN terminal=TRUE;first_points=CASE WHEN lead>0 THEN 2 ELSE 0 END;
 ELSIF n=18 THEN terminal=TRUE;first_points=1; END IF;
 ELSIF kind IN ('concession','award') THEN
 IF (e->>'after_hole')::int IS DISTINCT FROM n THEN RAISE EXCEPTION 'invalid terminal effective point' USING ERRCODE='23514'; END IF;
 IF kind='concession' THEN
 IF (e->>'communicated') IS DISTINCT FROM 'true' THEN RAISE EXCEPTION 'communicated concession required' USING ERRCODE='23514'; END IF;
 winner=e->>'conceding_player_id';first_points=CASE WHEN winner=first_player::text THEN 0 ELSE 2 END;
 ELSE winner=e->>'winner_player_id';first_points=CASE WHEN winner=first_player::text THEN 2 ELSE 0 END;
 END IF;
 IF winner IS NULL OR winner NOT IN (first_player::text,second_player::text) THEN RAISE EXCEPTION 'terminal player must be an opponent' USING ERRCODE='23514'; END IF;
 terminal=TRUE;
 ELSE RAISE EXCEPTION 'unknown accepted event' USING ERRCODE='23514';
 END IF;
 END LOOP;
 RETURN NEXT;
END $$;
CREATE FUNCTION singles_event_provenance(e JSONB,first_player UUID,second_player UUID,admin BOOLEAN) RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE b JSONB; kind TEXT; conceder TEXT; outcome TEXT;
BEGIN
 kind=e->>'type';b=e->'basis';outcome=e->>'outcome';
 IF kind='award' THEN RETURN admin AND COALESCE(length(btrim(e->>'reason')),0) BETWEEN 1 AND 1000; END IF;
 IF kind='concession' THEN RETURN (e->>'communicated') IS NOT DISTINCT FROM 'true'; END IF;
 IF kind IS DISTINCT FROM 'hole' THEN RETURN FALSE; END IF;
 kind=b->>'type';
 IF kind='organizer_ruling' THEN RETURN admin AND COALESCE(length(btrim(b->>'reason')),0) BETWEEN 1 AND 1000; END IF;
 IF kind='agreed_halve' THEN RETURN (b->>'play_begun') IS NOT DISTINCT FROM 'true' AND (b->>'mutual_agreement') IS NOT DISTINCT FROM 'true' AND outcome IS NOT DISTINCT FROM 'halved'; END IF;
 IF kind IN ('numeric','next_stroke_concession') THEN
 IF (b->>'agreed') IS DISTINCT FROM 'true' OR COALESCE((b->>'first_gross')::int,0) NOT BETWEEN 1 AND 20 OR COALESCE((b->>'second_gross')::int,0) NOT BETWEEN 1 AND 20 THEN RETURN FALSE; END IF;
 IF kind='numeric' THEN RETURN TRUE; END IF;
 END IF;
 IF kind IN ('hole_concession','next_stroke_concession') THEN
 conceder=b->>'conceding_player_id';
 RETURN COALESCE(conceder IN(first_player::text,second_player::text),FALSE) AND (b->>'communicated') IS NOT DISTINCT FROM 'true'
 AND (kind='next_stroke_concession' OR outcome IS NOT DISTINCT FROM CASE WHEN conceder=first_player::text THEN 'second' ELSE 'first' END);
 END IF;RETURN FALSE;
END $$;
CREATE FUNCTION validate_singles_command_result() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE command JSONB;kind TEXT;event JSONB;item JSONB;actual JSONB;derived RECORD;admin BOOLEAN;
BEGIN
 command=NULLIF(current_setting('app.match_command',true),'')::jsonb;kind=command->>'type';
 admin=singles_admin(NEW.round_id,NULLIF(current_setting('app.match_actor',true),'')::uuid,NULLIF(current_setting('app.match_session',true),'')::uuid);
 SELECT * INTO derived FROM singles_ledger_result(NEW.ledger,NEW.first_player_id,NEW.second_player_id);
 IF NEW.terminal IS DISTINCT FROM derived.terminal OR (NEW.confirmed AND (NEW.first_half_points IS DISTINCT FROM derived.first_points OR NEW.second_half_points IS DISTINCT FROM 2-derived.first_points)) THEN RAISE EXCEPTION 'aggregate must match accepted ledger' USING ERRCODE='23514'; END IF;
 IF kind IN ('note','clear_note') AND (NEW.terminal,NEW.confirmed,NEW.correction_pending,NEW.first_half_points,NEW.second_half_points) IS DISTINCT FROM (OLD.terminal,OLD.confirmed,OLD.correction_pending,OLD.first_half_points,OLD.second_half_points) THEN RAISE EXCEPTION 'notes cannot change match results' USING ERRCODE='23514'; END IF;
 IF kind='confirm' AND (NOT OLD.terminal OR NEW.correction_pending) THEN RAISE EXCEPTION 'confirmation requires already terminal ledger' USING ERRCODE='23514'; END IF;
 IF kind='report' THEN
 event=NEW.ledger->(jsonb_array_length(NEW.ledger)-1)->'event';
 IF event IS DISTINCT FROM command->'event' OR NOT COALESCE(singles_event_provenance(event,NEW.first_player_id,NEW.second_player_id,admin),FALSE) THEN RAISE EXCEPTION 'report provenance and authority required' USING ERRCODE='23514'; END IF;
 IF NEW.confirmed OR NEW.correction_pending IS DISTINCT FROM OLD.correction_pending THEN RAISE EXCEPTION 'report must remain unconfirmed' USING ERRCODE='23514'; END IF;
 END IF;
 IF kind='correct' THEN
 SELECT COALESCE(jsonb_agg(value->'id' ORDER BY ord),'[]'::jsonb) INTO actual FROM jsonb_array_elements(OLD.ledger) WITH ORDINALITY a(value,ord);
 IF actual IS DISTINCT FROM command->'superseded_event_ids' OR command->>'kind' NOT IN ('recording_error','organizer_ruling') THEN RAISE EXCEPTION 'correction must explicitly supersede accepted events' USING ERRCODE='23514'; END IF;
 SELECT COALESCE(jsonb_agg(value->'event' ORDER BY ord),'[]'::jsonb) INTO actual FROM jsonb_array_elements(NEW.ledger) WITH ORDINALITY a(value,ord);
 IF actual IS DISTINCT FROM command->'replacement' THEN RAISE EXCEPTION 'correction replacement mismatch' USING ERRCODE='23514'; END IF;
 FOR item IN SELECT value FROM jsonb_array_elements(actual) LOOP
 IF NOT COALESCE(singles_event_provenance(item,NEW.first_player_id,NEW.second_player_id,admin),FALSE) THEN RAISE EXCEPTION 'replacement provenance required' USING ERRCODE='23514'; END IF;
 END LOOP;
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER singles_command_result_guard BEFORE UPDATE ON singles_matches FOR EACH ROW EXECUTE FUNCTION validate_singles_command_result();
CREATE FUNCTION check_singles_audit_receipt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM singles_match_receipts WHERE match_id=NEW.match_id AND user_id=NEW.actor_id AND request_id=NEW.request_id AND applied_revision=NEW.revision) THEN RAISE EXCEPTION 'accepted command requires its immutable receipt' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER singles_audit_receipt_check AFTER INSERT ON singles_match_audits DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_singles_audit_receipt();
