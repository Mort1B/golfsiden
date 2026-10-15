CREATE FUNCTION test_pairing_wait() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.status<>'draft' OR NEW.updated_at<=OLD.updated_at OR
    EXISTS(SELECT 1 FROM teams WHERE round_id=NEW.id) OR
    EXISTS(SELECT 1 FROM team_memberships WHERE round_id=NEW.id) OR
    EXISTS(SELECT 1 FROM flights WHERE id='b1000000-0000-0000-0000-000000000033') OR
    (SELECT count(*) FROM flights WHERE round_id=NEW.id)<>2 OR
    (SELECT count(*) FROM flight_memberships WHERE round_id=NEW.id)<>3 OR
    NOT EXISTS(SELECT 1 FROM flights WHERE id='b1000000-0000-0000-0000-000000000031'
      AND name='Legacy A' AND starting_hole=4 AND tee_time='08:30') THEN
  RAISE EXCEPTION 'expected pending replacement and changed timestamp before late wait';
 END IF;
 PERFORM pg_advisory_xact_lock(260015,2);
 RETURN NEW;
END $$;
CREATE TRIGGER test_pairing_wait AFTER UPDATE OF updated_at ON rounds
 FOR EACH ROW WHEN(NEW.id='b1000000-0000-0000-0000-000000000002')
 EXECUTE FUNCTION test_pairing_wait();
