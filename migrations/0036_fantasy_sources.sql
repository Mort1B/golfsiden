-- Retained mutation generations invalidate empty-card and insert/delete fingerprints.
CREATE TABLE fantasy_owner_generations (
 round_id UUID NOT NULL REFERENCES rounds(id) ON DELETE RESTRICT,
 owner_kind TEXT NOT NULL CHECK(owner_kind IN ('player','team')),
 owner_id UUID NOT NULL,
 generation BIGINT NOT NULL CHECK(generation>0),
 PRIMARY KEY(round_id,owner_kind,owner_id)
);
CREATE FUNCTION fantasy_generation_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF pg_trigger_depth()<2 OR TG_OP='DELETE' THEN
  RAISE EXCEPTION 'source generations are maintained by source triggers' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_generation_guard BEFORE INSERT OR UPDATE OR DELETE ON fantasy_owner_generations FOR EACH ROW EXECUTE FUNCTION fantasy_generation_guard();
CREATE FUNCTION fantasy_source_changed() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE src JSONB; rid UUID; oid UUID; kind TEXT; second UUID;
BEGIN
 src=CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
 rid=(src->>'round_id')::UUID;
 IF NOT EXISTS(SELECT 1 FROM rounds r JOIN fantasy_games g ON g.tournament_id=r.tournament_id WHERE r.id=rid) THEN RETURN NULL; END IF;
 PERFORM acquire_score_round_lock(rid);
 IF TG_TABLE_NAME='singles_matches' THEN
  kind='player'; oid=(src->>'first_player_id')::UUID; second=(src->>'second_player_id')::UUID;
 ELSIF TG_TABLE_NAME='four_ball_inputs' THEN
  kind='team'; SELECT team_id INTO oid FROM team_memberships WHERE round_id=rid AND player_id=(src->>'player_id')::UUID;
 ELSE
  oid=(src->>'team_id')::UUID; kind='team';
  IF oid IS NULL THEN oid=(src->>'player_id')::UUID; kind='player'; END IF;
 END IF;
 IF oid IS NOT NULL THEN
  INSERT INTO fantasy_owner_generations(round_id,owner_kind,owner_id,generation) VALUES(rid,kind,oid,1)
  ON CONFLICT(round_id,owner_kind,owner_id) DO UPDATE SET generation=fantasy_owner_generations.generation+1;
 END IF;
 IF second IS NOT NULL THEN
  INSERT INTO fantasy_owner_generations(round_id,owner_kind,owner_id,generation) VALUES(rid,'player',second,1)
  ON CONFLICT(round_id,owner_kind,owner_id) DO UPDATE SET generation=fantasy_owner_generations.generation+1;
 END IF;
 RETURN NULL;
END;
$$;
CREATE TRIGGER fantasy_source_changed AFTER INSERT OR UPDATE OR DELETE ON scores FOR EACH ROW EXECUTE FUNCTION fantasy_source_changed();
CREATE TRIGGER fantasy_source_changed AFTER INSERT OR UPDATE OR DELETE ON four_ball_inputs FOR EACH ROW EXECUTE FUNCTION fantasy_source_changed();
CREATE TRIGGER fantasy_source_changed AFTER INSERT OR UPDATE OR DELETE ON stableford_inputs FOR EACH ROW EXECUTE FUNCTION fantasy_source_changed();
CREATE TRIGGER fantasy_source_changed AFTER INSERT OR UPDATE OR DELETE ON scorecard_confirmations FOR EACH ROW EXECUTE FUNCTION fantasy_source_changed();
CREATE TRIGGER fantasy_source_changed AFTER INSERT OR UPDATE OR DELETE ON singles_matches FOR EACH ROW EXECUTE FUNCTION fantasy_source_changed();
CREATE TABLE fantasy_dispositions (
 id UUID PRIMARY KEY,
 tournament_id UUID NOT NULL,
 round_id UUID NOT NULL,
 player_id UUID,
 team_id UUID,
 source_token BYTEA NOT NULL CHECK(octet_length(source_token)=32),
 disposed BOOLEAN NOT NULL,
 correction BOOLEAN NOT NULL,
 reason TEXT NOT NULL CHECK(length(btrim(reason)) BETWEEN 1 AND 500),
 actor_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 FOREIGN KEY(round_id,tournament_id) REFERENCES fantasy_rounds(round_id,tournament_id) ON DELETE RESTRICT,
 FOREIGN KEY(round_id,player_id) REFERENCES round_handicap_snapshots(round_id,player_id) ON DELETE RESTRICT,
 FOREIGN KEY(team_id,round_id,tournament_id) REFERENCES teams(id,round_id,tournament_id) ON DELETE RESTRICT,
 CHECK((player_id IS NOT NULL)::INTEGER+(team_id IS NOT NULL)::INTEGER=1)
);
CREATE INDEX fantasy_dispositions_owner ON fantasy_dispositions(round_id,player_id,team_id,created_at DESC,id);
CREATE TRIGGER fantasy_dispositions_immutable BEFORE UPDATE OR DELETE ON fantasy_dispositions FOR EACH ROW EXECUTE FUNCTION fantasy_immutable();
