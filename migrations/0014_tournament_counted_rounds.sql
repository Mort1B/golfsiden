BEGIN;

ALTER TABLE tournaments ADD COLUMN counted_rounds SMALLINT;

UPDATE tournaments SET counted_rounds = number_of_rounds;

ALTER TABLE tournaments
    ALTER COLUMN counted_rounds SET DEFAULT 1,
    ALTER COLUMN counted_rounds SET NOT NULL,
    ADD CONSTRAINT tournaments_counted_rounds_range
        CHECK (counted_rounds BETWEEN 1 AND number_of_rounds);

CREATE FUNCTION protect_tournament_counted_rounds() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    actor_text TEXT;
    actor_id UUID;
BEGIN
    IF NEW.counted_rounds IS NOT DISTINCT FROM OLD.counted_rounds THEN
        RETURN NEW;
    END IF;

    IF current_setting('app.tournament_configuration_tournament_id', true)
           IS DISTINCT FROM OLD.id::TEXT THEN
        RAISE EXCEPTION 'counted rounds must use the tournament configuration workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_configuration_context_required';
    END IF;

    actor_text = current_setting('app.tournament_configuration_user_id', true);
    BEGIN
        actor_id = actor_text::UUID;
    EXCEPTION WHEN invalid_text_representation THEN
        RAISE EXCEPTION 'counted rounds require a tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_configuration_admin_required';
    END;

    IF NOT EXISTS (
        SELECT 1 FROM tournament_memberships
        WHERE tournament_id = OLD.id AND user_id = actor_id AND role = 'admin'
    ) THEN
        RAISE EXCEPTION 'counted rounds require a tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_configuration_admin_required';
    END IF;

    IF OLD.status <> 'draft'
       OR EXISTS (SELECT 1 FROM tournament_handicap_locks WHERE tournament_id = OLD.id)
       OR EXISTS (SELECT 1 FROM rounds WHERE tournament_id = OLD.id AND status <> 'draft')
       OR EXISTS (SELECT 1 FROM round_handicap_snapshots WHERE tournament_id = OLD.id)
       OR EXISTS (SELECT 1 FROM round_team_handicap_snapshots WHERE tournament_id = OLD.id) THEN
        RAISE EXCEPTION 'counted rounds are frozen after round opening'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_configuration_locked';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tournaments_protect_counted_rounds
BEFORE UPDATE OF counted_rounds ON tournaments
FOR EACH ROW EXECUTE FUNCTION protect_tournament_counted_rounds();

COMMIT;
