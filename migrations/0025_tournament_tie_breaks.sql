BEGIN;

CREATE TYPE tournament_tie_break_policy AS ENUM ('shared_positions', 'final_round_score');
ALTER TABLE tournaments ADD COLUMN tie_break_policy tournament_tie_break_policy
    NOT NULL DEFAULT 'shared_positions';

-- Extend the existing exact-admin and permanent lifecycle guard without changing
-- creation contracts, historical scores, or previously selected counted rounds.
CREATE OR REPLACE FUNCTION protect_tournament_counted_round_configuration() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    actor_text TEXT;
    actor_id UUID;
BEGIN
    IF NEW.counted_rounds IS NOT DISTINCT FROM OLD.counted_rounds
       AND NEW.mandatory_round_id IS NOT DISTINCT FROM OLD.mandatory_round_id
       AND NEW.tie_break_policy IS NOT DISTINCT FROM OLD.tie_break_policy THEN
        RETURN NEW;
    END IF;

    IF current_setting('app.tournament_configuration_tournament_id', true)
           IS DISTINCT FROM OLD.id::TEXT THEN
        RAISE EXCEPTION 'counted-round configuration must use the tournament configuration workflow'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_configuration_context_required';
    END IF;

    actor_text = current_setting('app.tournament_configuration_user_id', true);
    BEGIN
        actor_id = actor_text::UUID;
    EXCEPTION WHEN invalid_text_representation THEN
        RAISE EXCEPTION 'counted-round configuration requires a tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_configuration_admin_required';
    END;

    IF NOT EXISTS (
        SELECT 1 FROM tournament_memberships
        WHERE tournament_id = OLD.id AND user_id = actor_id AND role = 'admin'
    ) THEN
        RAISE EXCEPTION 'counted-round configuration requires a tournament administrator'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_configuration_admin_required';
    END IF;

    IF OLD.status <> 'draft'
       OR EXISTS (SELECT 1 FROM tournament_handicap_locks WHERE tournament_id = OLD.id)
       OR EXISTS (SELECT 1 FROM rounds WHERE tournament_id = OLD.id AND status <> 'draft')
       OR EXISTS (SELECT 1 FROM round_handicap_snapshots WHERE tournament_id = OLD.id)
       OR EXISTS (SELECT 1 FROM round_team_handicap_snapshots WHERE tournament_id = OLD.id) THEN
        RAISE EXCEPTION 'counted-round configuration is frozen after tournament start or round opening'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_configuration_locked';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER tournaments_protect_counted_round_configuration ON tournaments;
CREATE TRIGGER tournaments_protect_counted_round_configuration
BEFORE UPDATE OF counted_rounds, mandatory_round_id, tie_break_policy ON tournaments
FOR EACH ROW EXECUTE FUNCTION protect_tournament_counted_round_configuration();

COMMIT;
