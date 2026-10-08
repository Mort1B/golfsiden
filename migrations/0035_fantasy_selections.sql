-- Private Fantasy state. History uses stable user/player identities, not memberships.
CREATE TABLE fantasy_games (
 tournament_id UUID PRIMARY KEY REFERENCES tournaments(id) ON DELETE RESTRICT,
 enabled BOOLEAN NOT NULL DEFAULT TRUE,
 rules_version SMALLINT NOT NULL DEFAULT 1 CHECK(rules_version=1),
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE fantasy_entries (
 tournament_id UUID NOT NULL REFERENCES fantasy_games(tournament_id) ON DELETE RESTRICT,
 user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
 entered_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY(tournament_id,user_id)
);
CREATE TABLE fantasy_rounds (
 round_id UUID PRIMARY KEY,
 tournament_id UUID NOT NULL REFERENCES fantasy_games(tournament_id) ON DELETE RESTRICT,
 deadline TIMESTAMPTZ,
 opened_at TIMESTAMPTZ,
 locked_at TIMESTAMPTZ,
 UNIQUE(round_id,tournament_id),
 FOREIGN KEY(round_id,tournament_id) REFERENCES rounds(id,tournament_id) ON DELETE RESTRICT
);
CREATE TABLE fantasy_lineups (
 id UUID PRIMARY KEY,
 tournament_id UUID NOT NULL,
 round_id UUID NOT NULL,
 user_id UUID NOT NULL,
 revision BIGINT NOT NULL CHECK(revision>0),
 first_player UUID NOT NULL,
 second_player UUID NOT NULL,
 third_player UUID NOT NULL,
 fourth_player UUID NOT NULL,
 captain UUID NOT NULL,
 origin TEXT NOT NULL CHECK(origin IN ('submitted','carried_forward')),
 source_round UUID,
 request_id UUID,
 expected_revision BIGINT,
 accepted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(round_id,user_id,revision),
 UNIQUE(id,round_id,tournament_id,user_id),
 UNIQUE(user_id,request_id),
 FOREIGN KEY(tournament_id,user_id) REFERENCES fantasy_entries(tournament_id,user_id) ON DELETE RESTRICT,
 FOREIGN KEY(round_id,tournament_id) REFERENCES fantasy_rounds(round_id,tournament_id) ON DELETE RESTRICT,
 FOREIGN KEY(source_round,tournament_id) REFERENCES fantasy_rounds(round_id,tournament_id) ON DELETE RESTRICT,
 FOREIGN KEY(tournament_id,first_player) REFERENCES tournament_players(tournament_id,player_id) ON DELETE RESTRICT,
 FOREIGN KEY(tournament_id,second_player) REFERENCES tournament_players(tournament_id,player_id) ON DELETE RESTRICT,
 FOREIGN KEY(tournament_id,third_player) REFERENCES tournament_players(tournament_id,player_id) ON DELETE RESTRICT,
 FOREIGN KEY(tournament_id,fourth_player) REFERENCES tournament_players(tournament_id,player_id) ON DELETE RESTRICT,
 CHECK(first_player<>second_player AND first_player<>third_player AND first_player<>fourth_player AND second_player<>third_player AND second_player<>fourth_player AND third_player<>fourth_player),
 CHECK(captain IN (first_player,second_player,third_player,fourth_player)),
 CHECK((origin='submitted' AND request_id IS NOT NULL AND expected_revision IS NOT NULL AND expected_revision>=0 AND source_round IS NULL) OR (origin='carried_forward' AND request_id IS NULL AND expected_revision IS NULL AND source_round IS NOT NULL))
);
CREATE TABLE fantasy_selections (
 round_id UUID NOT NULL,
 tournament_id UUID NOT NULL,
 user_id UUID NOT NULL,
 lineup_id UUID,
 state TEXT NOT NULL CHECK(state IN ('draft','locked','missed','invalid','not_participating')),
 locked_at TIMESTAMPTZ,
 PRIMARY KEY(round_id,user_id),
 FOREIGN KEY(round_id,tournament_id) REFERENCES fantasy_rounds(round_id,tournament_id) ON DELETE RESTRICT,
 FOREIGN KEY(tournament_id,user_id) REFERENCES fantasy_entries(tournament_id,user_id) ON DELETE RESTRICT,
 FOREIGN KEY(lineup_id,round_id,tournament_id,user_id) REFERENCES fantasy_lineups(id,round_id,tournament_id,user_id) ON DELETE RESTRICT,
 CHECK((state='draft' AND lineup_id IS NOT NULL AND locked_at IS NULL) OR (state='locked' AND lineup_id IS NOT NULL AND locked_at IS NOT NULL) OR (state IN ('missed','invalid','not_participating') AND lineup_id IS NULL AND locked_at IS NOT NULL))
);
CREATE TABLE fantasy_configuration_audits (
 id BIGSERIAL PRIMARY KEY,
 tournament_id UUID NOT NULL REFERENCES fantasy_games(tournament_id) ON DELETE RESTRICT,
 actor_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
 action TEXT NOT NULL,
 round_id UUID,
 FOREIGN KEY(round_id,tournament_id) REFERENCES fantasy_rounds(round_id,tournament_id) ON DELETE RESTRICT,
 deadline TIMESTAMPTZ,
 created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE FUNCTION fantasy_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'Fantasy history is immutable' USING ERRCODE='23514',CONSTRAINT='fantasy_immutable';
END;
$$;
CREATE TRIGGER fantasy_lineups_immutable BEFORE UPDATE OR DELETE ON fantasy_lineups FOR EACH ROW EXECUTE FUNCTION fantasy_immutable();
CREATE TRIGGER fantasy_entries_immutable BEFORE UPDATE OR DELETE ON fantasy_entries FOR EACH ROW EXECUTE FUNCTION fantasy_immutable();
CREATE TRIGGER fantasy_audits_immutable BEFORE UPDATE OR DELETE ON fantasy_configuration_audits FOR EACH ROW EXECUTE FUNCTION fantasy_immutable();
CREATE FUNCTION fantasy_selection_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' OR OLD.locked_at IS NOT NULL THEN
  RAISE EXCEPTION 'locked Fantasy selections are immutable' USING ERRCODE='23514',CONSTRAINT='fantasy_immutable';
 END IF;
 IF NEW.round_id<>OLD.round_id OR NEW.tournament_id<>OLD.tournament_id OR NEW.user_id<>OLD.user_id THEN
  RAISE EXCEPTION 'Fantasy selection identity is immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_selection_guard BEFORE UPDATE OR DELETE ON fantasy_selections FOR EACH ROW EXECUTE FUNCTION fantasy_selection_guard();
-- Serialize eligibility histories and lazy closure. Round locks are acquired first
-- by repository writers; history triggers use NOWAIT to avoid reverse-lock waits.
CREATE FUNCTION fantasy_lock_tournament(t UUID) RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
 PERFORM id FROM rounds WHERE tournament_id=t AND EXISTS(SELECT 1 FROM fantasy_games WHERE tournament_id=t AND enabled) ORDER BY id FOR UPDATE NOWAIT;
EXCEPTION WHEN lock_not_available THEN
 RAISE EXCEPTION 'Fantasy eligibility changed concurrently; retry' USING ERRCODE='23514',CONSTRAINT='fantasy_history_busy';
END;
$$;
CREATE TABLE fantasy_eligibility_history (
 id BIGSERIAL PRIMARY KEY,
 tournament_id UUID NOT NULL,
 player_id UUID NOT NULL,
 eligible BOOLEAN NOT NULL,
 effective_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX fantasy_eligibility_asof ON fantasy_eligibility_history(tournament_id,player_id,effective_at DESC,id DESC);
CREATE TABLE fantasy_membership_history (
 id BIGSERIAL PRIMARY KEY,
 tournament_id UUID NOT NULL,
 user_id UUID NOT NULL,
 present BOOLEAN NOT NULL,
 effective_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX fantasy_membership_asof ON fantasy_membership_history(tournament_id,user_id,effective_at DESC,id DESC);
INSERT INTO fantasy_eligibility_history(tournament_id,player_id,eligible) SELECT tp.tournament_id,tp.player_id,tp.status='active' AND p.active FROM tournament_players tp JOIN players p ON p.id=tp.player_id;
INSERT INTO fantasy_membership_history(tournament_id,user_id,present) SELECT tournament_id,user_id,true FROM tournament_memberships;
CREATE TRIGGER fantasy_eligibility_immutable BEFORE UPDATE OR DELETE ON fantasy_eligibility_history FOR EACH ROW EXECUTE FUNCTION fantasy_immutable();
CREATE TRIGGER fantasy_membership_immutable BEFORE UPDATE OR DELETE ON fantasy_membership_history FOR EACH ROW EXECUTE FUNCTION fantasy_immutable();
CREATE FUNCTION fantasy_roster_history() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE t UUID; p UUID; ok BOOLEAN;
BEGIN
 t=CASE WHEN TG_OP='DELETE' THEN OLD.tournament_id ELSE NEW.tournament_id END;
 p=CASE WHEN TG_OP='DELETE' THEN OLD.player_id ELSE NEW.player_id END;
 PERFORM fantasy_lock_tournament(t);
 PERFORM id FROM players WHERE id=p FOR SHARE;
 ok=FALSE;
 IF TG_OP<>'DELETE' THEN SELECT NEW.status='active' AND active INTO ok FROM players WHERE id=p FOR SHARE; END IF;
 INSERT INTO fantasy_eligibility_history(tournament_id,player_id,eligible) VALUES(t,p,COALESCE(ok,FALSE));
 RETURN CASE WHEN TG_OP='DELETE' THEN OLD ELSE NEW END;
END;
$$;
CREATE TRIGGER fantasy_roster_history AFTER INSERT OR UPDATE OF status OR DELETE ON tournament_players FOR EACH ROW EXECUTE FUNCTION fantasy_roster_history();
CREATE FUNCTION fantasy_player_history() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE entry RECORD;
BEGIN
 IF OLD.active IS NOT DISTINCT FROM NEW.active THEN RETURN NEW; END IF;
 PERFORM r.id FROM rounds r JOIN tournament_players tp ON tp.tournament_id=r.tournament_id WHERE tp.player_id=NEW.id AND EXISTS(SELECT 1 FROM fantasy_games g WHERE g.tournament_id=r.tournament_id AND g.enabled) ORDER BY r.id FOR UPDATE OF r NOWAIT;
 FOR entry IN SELECT tournament_id,status FROM tournament_players WHERE player_id=NEW.id ORDER BY tournament_id LOOP
  INSERT INTO fantasy_eligibility_history(tournament_id,player_id,eligible) VALUES(entry.tournament_id,NEW.id,NEW.active AND entry.status='active');
 END LOOP;
 RETURN NEW;
EXCEPTION WHEN lock_not_available THEN
 RAISE EXCEPTION 'Fantasy eligibility changed concurrently; retry' USING ERRCODE='23514',CONSTRAINT='fantasy_history_busy';
END;
$$;
CREATE TRIGGER fantasy_player_history AFTER UPDATE OF active ON players FOR EACH ROW EXECUTE FUNCTION fantasy_player_history();
CREATE FUNCTION fantasy_member_history() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE t UUID; u UUID;
BEGIN
 IF TG_OP='UPDATE' THEN
  IF NEW.tournament_id=OLD.tournament_id AND NEW.user_id=OLD.user_id THEN RETURN NEW; END IF;
  -- Preserve existing membership moves and recovery authority serialization.
  -- NOWAIT round locks reject reverse-order contention instead of deadlocking.
  PERFORM fantasy_lock_tournament(OLD.tournament_id);
  PERFORM fantasy_lock_tournament(NEW.tournament_id);
  INSERT INTO fantasy_membership_history(tournament_id,user_id,present) VALUES(OLD.tournament_id,OLD.user_id,false);
  INSERT INTO fantasy_membership_history(tournament_id,user_id,present) VALUES(NEW.tournament_id,NEW.user_id,true);
  RETURN NEW;
 END IF;
 t=CASE WHEN TG_OP='DELETE' THEN OLD.tournament_id ELSE NEW.tournament_id END;
 u=CASE WHEN TG_OP='DELETE' THEN OLD.user_id ELSE NEW.user_id END;
 PERFORM fantasy_lock_tournament(t);
 INSERT INTO fantasy_membership_history(tournament_id,user_id,present) VALUES(t,u,TG_OP<>'DELETE');
 RETURN CASE WHEN TG_OP='DELETE' THEN OLD ELSE NEW END;
END;
$$;
CREATE TRIGGER fantasy_member_history AFTER INSERT OR UPDATE OF tournament_id,user_id OR DELETE ON tournament_memberships FOR EACH ROW EXECUTE FUNCTION fantasy_member_history();
CREATE FUNCTION fantasy_round_opened() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.status='open' AND OLD.status='draft' THEN
  UPDATE fantasy_rounds SET opened_at=clock_timestamp() WHERE round_id=NEW.id AND opened_at IS NULL;
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_round_opened BEFORE UPDATE OF status ON rounds FOR EACH ROW EXECUTE FUNCTION fantasy_round_opened();
CREATE FUNCTION fantasy_round_created() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO fantasy_rounds(round_id,tournament_id) SELECT NEW.id,NEW.tournament_id FROM fantasy_games WHERE tournament_id=NEW.tournament_id;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_round_created AFTER INSERT ON rounds FOR EACH ROW EXECUTE FUNCTION fantasy_round_created();
-- Histories may only originate inside the roster/membership trigger chain.
CREATE FUNCTION fantasy_history_insert() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF pg_trigger_depth()<2 THEN RAISE EXCEPTION 'history requires a source mutation' USING ERRCODE='23514'; END IF;
 NEW.effective_at=clock_timestamp(); RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_history_insert BEFORE INSERT ON fantasy_eligibility_history FOR EACH ROW EXECUTE FUNCTION fantasy_history_insert();
CREATE TRIGGER fantasy_history_insert BEFORE INSERT ON fantasy_membership_history FOR EACH ROW EXECUTE FUNCTION fantasy_history_insert();
