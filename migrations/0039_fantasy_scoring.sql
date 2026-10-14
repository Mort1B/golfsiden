-- Rules are game-wide, never per-round. Stop concurrent round openings, window
-- changes/materialization and configuration while establishing the upgrade cut.
-- These table locks and the cut occur in this migration's single transaction.
LOCK TABLE rounds, fantasy_games, fantasy_rounds, fantasy_selections IN ACCESS EXCLUSIVE MODE;
ALTER TABLE fantasy_games DROP CONSTRAINT fantasy_games_rules_version_check;
ALTER TABLE fantasy_games ADD CONSTRAINT fantasy_games_rules_version_check CHECK(rules_version IN (1,2));
ALTER TABLE fantasy_games ALTER COLUMN rules_version SET DEFAULT 2;

-- Only the migration can change a game's version. Restore the runtime
-- trigger in this same transaction; immutable version protection survives release.
DROP TRIGGER fantasy_game_guard ON fantasy_games;
DO $$
DECLARE upgrade_cut TIMESTAMPTZ := clock_timestamp();
BEGIN
 UPDATE fantasy_games g SET rules_version=2
 WHERE g.rules_version=1
 AND NOT EXISTS (SELECT 1 FROM rounds r WHERE r.tournament_id=g.tournament_id AND r.status<>'draft')
 AND NOT EXISTS (SELECT 1 FROM fantasy_rounds f WHERE f.tournament_id=g.tournament_id
   AND (f.locked_at IS NOT NULL OR f.opened_at IS NOT NULL OR f.deadline<=upgrade_cut))
 AND NOT EXISTS (SELECT 1 FROM fantasy_selections s WHERE s.tournament_id=g.tournament_id
   AND (s.locked_at IS NOT NULL OR s.state<>'draft'));
END;
$$;
CREATE OR REPLACE FUNCTION fantasy_game_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Fantasy games retain history' USING ERRCODE='23514'; END IF;
 PERFORM fantasy_lock_tournament(NEW.tournament_id);
 PERFORM id FROM tournaments WHERE id=NEW.tournament_id FOR UPDATE;
 PERFORM p.id FROM tournament_players tp JOIN players p ON p.id=tp.player_id WHERE tp.tournament_id=NEW.tournament_id ORDER BY p.id FOR SHARE OF tp,p;
 PERFORM user_id FROM tournament_memberships WHERE tournament_id=NEW.tournament_id ORDER BY user_id FOR SHARE;
 IF EXISTS(SELECT 1 FROM rounds WHERE tournament_id=NEW.tournament_id AND status<>'draft') OR EXISTS(SELECT 1 FROM fantasy_rounds WHERE tournament_id=NEW.tournament_id AND (locked_at IS NOT NULL OR least(deadline,opened_at)<=clock_timestamp())) THEN
  RAISE EXCEPTION 'Fantasy configuration is closed' USING ERRCODE='23514';
 END IF;
 IF TG_OP='UPDATE' AND (NEW.tournament_id<>OLD.tournament_id OR NEW.created_at<>OLD.created_at OR NEW.rules_version<>OLD.rules_version) THEN RAISE EXCEPTION 'game identity is immutable' USING ERRCODE='23514'; END IF;
 IF TG_OP='INSERT' THEN
  IF NEW.rules_version<>2 THEN RAISE EXCEPTION 'new Fantasy games require current rules' USING ERRCODE='23514'; END IF;
  NEW.created_at=clock_timestamp();
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_game_guard BEFORE INSERT OR UPDATE OR DELETE ON fantasy_games FOR EACH ROW EXECUTE FUNCTION fantasy_game_guard();
