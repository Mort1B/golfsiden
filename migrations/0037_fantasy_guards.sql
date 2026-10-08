CREATE FUNCTION fantasy_game_guard() RETURNS trigger LANGUAGE plpgsql AS $$
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
 IF TG_OP='INSERT' THEN NEW.created_at=clock_timestamp(); END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_game_guard BEFORE INSERT OR UPDATE OR DELETE ON fantasy_games FOR EACH ROW EXECUTE FUNCTION fantasy_game_guard();
CREATE FUNCTION fantasy_window_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Fantasy windows retain history' USING ERRCODE='23514'; END IF;
 IF TG_OP='UPDATE' THEN
  IF OLD.opened_at IS NULL AND NEW.opened_at IS NOT NULL THEN
   IF pg_trigger_depth()<2 THEN RAISE EXCEPTION 'opening timestamp requires round opening' USING ERRCODE='23514'; END IF;
   NEW.opened_at=clock_timestamp();
  END IF;
  IF (OLD.locked_at IS NOT NULL AND NOT (pg_trigger_depth()>1 AND OLD.opened_at IS NULL AND NEW.opened_at IS NOT NULL AND NEW.locked_at IS NOT DISTINCT FROM OLD.locked_at AND NEW.deadline IS NOT DISTINCT FROM OLD.deadline)) OR NEW.round_id<>OLD.round_id OR NEW.tournament_id<>OLD.tournament_id OR (OLD.opened_at IS NOT NULL AND NEW.opened_at IS DISTINCT FROM OLD.opened_at) THEN RAISE EXCEPTION 'Fantasy window is immutable' USING ERRCODE='23514'; END IF;
  IF NEW.deadline IS DISTINCT FROM OLD.deadline AND (least(OLD.deadline,OLD.opened_at)<=clock_timestamp() OR NEW.deadline<=clock_timestamp() OR OLD.opened_at IS NOT NULL) THEN RAISE EXCEPTION 'Fantasy deadline is closed' USING ERRCODE='23514'; END IF;
  IF NEW.locked_at IS NOT NULL AND (NEW.locked_at IS DISTINCT FROM least(OLD.deadline,OLD.opened_at) OR NEW.locked_at>clock_timestamp()) THEN RAISE EXCEPTION 'invalid effective lock' USING ERRCODE='23514'; END IF;
 END IF;
 IF TG_OP='INSERT' AND (NEW.opened_at IS NOT NULL OR NEW.locked_at IS NOT NULL OR NEW.deadline<=clock_timestamp()) THEN RAISE EXCEPTION 'invalid initial window' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_window_guard BEFORE INSERT OR UPDATE OR DELETE ON fantasy_rounds FOR EACH ROW EXECUTE FUNCTION fantasy_window_guard();
CREATE FUNCTION fantasy_entry_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 NEW.entered_at=clock_timestamp();
 IF NOT EXISTS(SELECT 1 FROM tournament_memberships WHERE tournament_id=NEW.tournament_id AND user_id=NEW.user_id) OR NOT EXISTS(SELECT 1 FROM fantasy_games WHERE tournament_id=NEW.tournament_id AND enabled) THEN RAISE EXCEPTION 'entry requires current membership and enabled game' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_entry_guard BEFORE INSERT ON fantasy_entries FOR EACH ROW EXECUTE FUNCTION fantasy_entry_guard();
CREATE FUNCTION fantasy_lineup_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE f fantasy_rounds%ROWTYPE; effective TIMESTAMPTZ;
BEGIN
 PERFORM acquire_score_round_lock(NEW.round_id);
 SELECT * INTO f FROM fantasy_rounds WHERE round_id=NEW.round_id;
 effective=least(f.deadline,f.opened_at);
 IF f.locked_at IS NOT NULL THEN RAISE EXCEPTION 'Fantasy selections are locked' USING ERRCODE='23514'; END IF;
 IF NEW.origin='submitted' THEN
  IF effective<=clock_timestamp() THEN RAISE EXCEPTION 'selection deadline passed' USING ERRCODE='23514'; END IF;
  NEW.accepted_at=clock_timestamp();
 ELSE
  IF effective IS NULL OR effective>clock_timestamp() OR NEW.accepted_at IS DISTINCT FROM effective THEN RAISE EXCEPTION 'invalid carry deadline' USING ERRCODE='23514'; END IF;
  IF NOT EXISTS(SELECT 1 FROM fantasy_selections s JOIN fantasy_lineups l ON l.id=s.lineup_id JOIN rounds prior ON prior.id=s.round_id JOIN rounds target ON target.id=NEW.round_id WHERE s.round_id=NEW.source_round AND s.tournament_id=NEW.tournament_id AND s.user_id=NEW.user_id AND s.state='locked' AND s.locked_at<=effective AND prior.round_number<target.round_number AND l.first_player=NEW.first_player AND l.second_player=NEW.second_player AND l.third_player=NEW.third_player AND l.fourth_player=NEW.fourth_player AND l.captain=NEW.captain) THEN RAISE EXCEPTION 'invalid carry source' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_lineup_guard BEFORE INSERT ON fantasy_lineups FOR EACH ROW EXECUTE FUNCTION fantasy_lineup_guard();
CREATE FUNCTION fantasy_selection_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE f fantasy_rounds%ROWTYPE;
BEGIN
 PERFORM acquire_score_round_lock(NEW.round_id);
 SELECT * INTO f FROM fantasy_rounds WHERE round_id=NEW.round_id;
 IF f.locked_at IS NOT NULL THEN RAISE EXCEPTION 'Fantasy round already finalized' USING ERRCODE='23514'; END IF;
 IF NEW.state='draft' THEN
  IF least(f.deadline,f.opened_at)<=clock_timestamp() THEN RAISE EXCEPTION 'draft deadline passed' USING ERRCODE='23514'; END IF;
 ELSE
  IF least(f.deadline,f.opened_at) IS NULL OR NEW.locked_at IS DISTINCT FROM least(f.deadline,f.opened_at) OR NEW.locked_at>clock_timestamp() THEN RAISE EXCEPTION 'invalid selection close' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_selection_state_guard BEFORE INSERT OR UPDATE ON fantasy_selections FOR EACH ROW EXECUTE FUNCTION fantasy_selection_state_guard();
CREATE FUNCTION fantasy_disposition_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE status round_status; actor UUID; session UUID;
BEGIN
 PERFORM acquire_score_round_lock(NEW.round_id);
 SELECT r.status INTO status FROM rounds r WHERE id=NEW.round_id;
 actor=NULLIF(current_setting('app.fantasy_actor',true),'')::UUID;
 session=NULLIF(current_setting('app.fantasy_session',true),'')::UUID;
 IF actor IS DISTINCT FROM NEW.actor_id OR NOT EXISTS(SELECT 1 FROM tournament_memberships m JOIN users u ON u.id=m.user_id JOIN user_sessions s ON s.user_id=u.id WHERE m.tournament_id=NEW.tournament_id AND m.user_id=actor AND m.role='admin' AND s.id=session AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND s.credential_generation=u.credential_generation) THEN RAISE EXCEPTION 'disposition requires exact administrator' USING ERRCODE='23514'; END IF;
 IF status='draft' OR ((status='locked' OR NOT NEW.disposed OR EXISTS(SELECT 1 FROM fantasy_dispositions d WHERE d.round_id=NEW.round_id AND (d.player_id=NEW.player_id OR d.team_id=NEW.team_id))) AND NOT NEW.correction) THEN RAISE EXCEPTION 'explicit correction is required' USING ERRCODE='23514'; END IF;
 IF NEW.disposed AND (EXISTS(SELECT 1 FROM scorecard_confirmations c WHERE c.round_id=NEW.round_id AND (c.player_id=NEW.player_id OR c.team_id=NEW.team_id)) OR EXISTS(SELECT 1 FROM singles_matches m WHERE m.round_id=NEW.round_id AND (m.first_player_id=NEW.player_id OR m.second_player_id=NEW.player_id) AND terminal)) THEN RAISE EXCEPTION 'accepted result takes precedence' USING ERRCODE='23514'; END IF;
 NEW.created_at=clock_timestamp();RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_disposition_guard BEFORE INSERT ON fantasy_dispositions FOR EACH ROW EXECUTE FUNCTION fantasy_disposition_guard();

CREATE FUNCTION fantasy_history_identity_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.tournament_id<>OLD.tournament_id OR NEW.player_id<>OLD.player_id THEN
  RAISE EXCEPTION 'roster identity requires deletion and insertion' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_history_identity_guard BEFORE UPDATE ON tournament_players FOR EACH ROW EXECUTE FUNCTION fantasy_history_identity_guard();
CREATE FUNCTION fantasy_round_order_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.round_number<>OLD.round_number AND EXISTS(SELECT 1 FROM fantasy_rounds WHERE round_id=OLD.id AND (locked_at IS NOT NULL OR least(deadline,opened_at)<=clock_timestamp())) THEN RAISE EXCEPTION 'locked Fantasy round order is preserved' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER fantasy_round_order_guard BEFORE UPDATE OF round_number ON rounds FOR EACH ROW EXECUTE FUNCTION fantasy_round_order_guard();
