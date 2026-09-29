-- Detail edits are separate from scoring configuration and lifecycle transitions.
-- Existing rows are not rewritten. Legacy date inconsistencies must be corrected
-- by choosing a containing range; round dates are never moved implicitly.
CREATE FUNCTION guard_tournament_details() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE actor UUID;
BEGIN
    IF (NEW.name, NEW.description, NEW.start_date, NEW.end_date)
       IS NOT DISTINCT FROM (OLD.name, OLD.description, OLD.start_date, OLD.end_date) THEN
        RETURN NEW;
    END IF;
    -- A fixed transaction snapshot can miss a round-date edit committed before
    -- the parent lock. The authorized workflow requires fresh statement snapshots.
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'details require read committed isolation'
            USING ERRCODE='23514', CONSTRAINT='tournament_details_isolation';
    END IF;
    IF OLD.status <> 'draft' OR NEW.status <> 'draft' THEN
        RAISE EXCEPTION 'only draft tournaments can change details'
            USING ERRCODE='23514', CONSTRAINT='tournament_details_locked';
    END IF;
    IF current_setting('app.tournament_details_id', true) IS DISTINCT FROM OLD.id::text THEN
        RAISE EXCEPTION 'details require the authorized workflow'
            USING ERRCODE='23514', CONSTRAINT='tournament_details_context';
    END IF;
    SELECT s.user_id INTO actor FROM user_sessions s JOIN users u ON u.id=s.user_id
    WHERE s.id::text=current_setting('app.tournament_details_session_id', true)
      AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
      AND s.credential_generation=u.credential_generation
    FOR SHARE OF s, u;
    IF actor IS NULL THEN
        RAISE EXCEPTION 'details require an active session'
            USING ERRCODE='23514', CONSTRAINT='tournament_details_session';
    END IF;
    PERFORM 1 FROM tournament_memberships WHERE tournament_id=OLD.id AND user_id=actor AND role='admin' FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'details require an exact tournament administrator'
            USING ERRCODE='23514', CONSTRAINT='tournament_details_admin';
    END IF;
    IF btrim(NEW.name)='' OR octet_length(NEW.name)>120 OR octet_length(NEW.description)>2000 THEN
        RAISE EXCEPTION 'invalid detail text' USING ERRCODE='23514', CONSTRAINT='tournament_details_text';
    END IF;
    IF NEW.end_date < NEW.start_date OR EXISTS (
        SELECT 1 FROM rounds WHERE tournament_id=OLD.id AND (round_date < NEW.start_date OR round_date > NEW.end_date)
    ) THEN
        RAISE EXCEPTION 'dates must contain every round' USING ERRCODE='23514', CONSTRAINT='tournament_details_round_dates';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER tournaments_guard_details BEFORE UPDATE OF name, description, start_date, end_date ON tournaments
FOR EACH ROW EXECUTE FUNCTION guard_tournament_details();

-- Serialize date changes/inserts against parent detail edits. Ordinary course,
-- score and lifecycle updates do not enter this trigger.
CREATE FUNCTION guard_round_tournament_dates() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE first_day DATE; last_day DATE;
BEGIN
    IF TG_OP='UPDATE' AND (NEW.tournament_id, NEW.round_date) IS NOT DISTINCT FROM (OLD.tournament_id, OLD.round_date) THEN RETURN NEW; END IF;
    SELECT start_date, end_date INTO first_day, last_day FROM tournaments WHERE id=NEW.tournament_id FOR SHARE;
    IF FOUND AND (NEW.round_date < first_day OR NEW.round_date > last_day) THEN
        RAISE EXCEPTION 'round date must be within tournament dates'
            USING ERRCODE='23514', CONSTRAINT='round_tournament_dates';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER rounds_guard_tournament_dates BEFORE INSERT OR UPDATE OF tournament_id, round_date ON rounds
FOR EACH ROW EXECUTE FUNCTION guard_round_tournament_dates();
