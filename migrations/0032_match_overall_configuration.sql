-- Match play has no overall contribution. Existing numeric N values are retained.
ALTER TABLE tournaments ALTER COLUMN counted_rounds DROP NOT NULL;
-- Schemas 30/31 could hold match plans using the old numeric-N contract. Only
-- those new-format plans are normalized: all-match has no overall; mixed plans
-- cap N to eligible formats and remove an ineligible mandatory match. Existing
-- non-match configurations, historical scores, snapshots and receipts are untouched.
ALTER TABLE tournaments DISABLE TRIGGER tournaments_protect_counted_round_configuration;
WITH formats AS (
 SELECT tournament_id,count(*) FILTER(WHERE scoring_format::text<>'singles_match_play') AS eligible
 FROM rounds GROUP BY tournament_id HAVING bool_or(scoring_format::text='singles_match_play')
)
UPDATE tournaments t SET
 counted_rounds=CASE WHEN f.eligible=0 THEN NULL ELSE LEAST(t.counted_rounds,f.eligible)::smallint END,
 mandatory_round_id=CASE WHEN f.eligible=0 OR EXISTS(SELECT 1 FROM rounds r WHERE r.id=t.mandatory_round_id AND r.scoring_format::text='singles_match_play') THEN NULL ELSE t.mandatory_round_id END
FROM formats f WHERE f.tournament_id=t.id;
ALTER TABLE tournaments ENABLE TRIGGER tournaments_protect_counted_round_configuration;

ALTER TABLE tournaments ADD CONSTRAINT match_only_mandatory_absent CHECK(counted_rounds IS NOT NULL OR mandatory_round_id IS NULL);
-- An internal generation row turns concurrent snapshot-isolation validations
-- into write conflicts without changing tournament timestamps or public data.
CREATE TABLE tournament_overall_configuration_guards (
 tournament_id UUID PRIMARY KEY REFERENCES tournaments(id) ON DELETE CASCADE,
 generation BIGINT NOT NULL DEFAULT 0 CHECK(generation>=0)
);
INSERT INTO tournament_overall_configuration_guards(tournament_id) SELECT id FROM tournaments;
CREATE FUNCTION initialize_overall_configuration_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO tournament_overall_configuration_guards(tournament_id) VALUES(NEW.id);
 RETURN NEW;
END $$;
CREATE TRIGGER tournament_overall_guard_initialize AFTER INSERT ON tournaments FOR EACH ROW EXECUTE FUNCTION initialize_overall_configuration_guard();
CREATE FUNCTION validate_overall_eligible_configuration(target UUID) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE t tournaments%ROWTYPE;eligible BIGINT;matches BIGINT;
BEGIN
 SELECT * INTO t FROM tournaments WHERE id=target FOR UPDATE;
 IF NOT FOUND THEN RETURN; END IF;
 UPDATE tournament_overall_configuration_guards SET generation=generation+1 WHERE tournament_id=target;
 IF NOT FOUND THEN RAISE EXCEPTION 'overall configuration guard missing' USING ERRCODE='23514'; END IF;
 SELECT count(*) FILTER(WHERE scoring_format::text<>'singles_match_play'),count(*) FILTER(WHERE scoring_format::text='singles_match_play') INTO eligible,matches FROM rounds WHERE tournament_id=target;
 IF t.mandatory_round_id IS NOT NULL AND EXISTS(SELECT 1 FROM rounds WHERE id=t.mandatory_round_id AND scoring_format::text='singles_match_play') THEN RAISE EXCEPTION 'mandatory round must contribute to overall results' USING ERRCODE='23514',CONSTRAINT='tournament_mandatory_round_ineligible'; END IF;
 -- Legacy tools may assemble draft stroke plans across transactions. Their
 -- scheduled-count constraint remains intact. Match and nullable plans must
 -- match the currently configured eligible formats at transaction commit.
 IF matches>0 OR t.counted_rounds IS NULL THEN
 IF (eligible=0 AND (t.counted_rounds IS NOT NULL OR t.mandatory_round_id IS NOT NULL)) OR (eligible>0 AND (t.counted_rounds IS NULL OR t.counted_rounds<1 OR t.counted_rounds>eligible)) THEN
 RAISE EXCEPTION 'counted rounds must match overall-eligible formats' USING ERRCODE='23514',CONSTRAINT='tournament_overall_eligible_count'; END IF;
 END IF;
END $$;
CREATE FUNCTION check_overall_eligible_configuration() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_TABLE_NAME='tournaments' THEN
 IF TG_OP='UPDATE' AND (NEW.counted_rounds,NEW.mandatory_round_id,NEW.number_of_rounds) IS NOT DISTINCT FROM (OLD.counted_rounds,OLD.mandatory_round_id,OLD.number_of_rounds) THEN RETURN NULL; END IF;
 PERFORM validate_overall_eligible_configuration(NEW.id);
 ELSE
 IF TG_OP='UPDATE' AND (NEW.scoring_format,NEW.tournament_id) IS NOT DISTINCT FROM (OLD.scoring_format,OLD.tournament_id) THEN RETURN NULL; END IF;
 IF TG_OP<>'DELETE' THEN PERFORM validate_overall_eligible_configuration(NEW.tournament_id); END IF;
 IF TG_OP<>'INSERT' THEN PERFORM validate_overall_eligible_configuration(OLD.tournament_id); END IF;
 END IF;RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER tournament_overall_configuration_check AFTER INSERT OR UPDATE ON tournaments DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_overall_eligible_configuration();
CREATE CONSTRAINT TRIGGER round_overall_configuration_check AFTER INSERT OR UPDATE OR DELETE ON rounds DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_overall_eligible_configuration();
CREATE FUNCTION guard_match_only_result_share() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (SELECT counted_rounds IS NULL FROM tournaments WHERE id=NEW.tournament_id) THEN RAISE EXCEPTION 'overall result sharing is unavailable' USING ERRCODE='23514',CONSTRAINT='result_share_overall_unavailable'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER match_only_result_share_guard BEFORE INSERT ON tournament_result_shares FOR EACH ROW EXECUTE FUNCTION guard_match_only_result_share();
