CREATE TABLE player_claim_grants (
    id UUID PRIMARY KEY,
    tournament_id UUID NOT NULL,
    player_id UUID NOT NULL,
    token_hash BYTEA NOT NULL CHECK (octet_length(token_hash) = 32),
    created_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp() + interval '7 days',
    revoked_at TIMESTAMPTZ,
    claimed_at TIMESTAMPTZ,
    claimed_by_user_id UUID REFERENCES users(id) ON DELETE RESTRICT,
    claimed_transaction XID8,
    FOREIGN KEY (tournament_id, player_id) REFERENCES tournament_players(tournament_id, player_id) ON DELETE RESTRICT,
    CHECK ((claimed_at IS NULL AND claimed_by_user_id IS NULL AND claimed_transaction IS NULL)
        OR (claimed_at IS NOT NULL AND claimed_by_user_id IS NOT NULL AND claimed_transaction IS NOT NULL))
);
CREATE UNIQUE INDEX player_claim_grants_active ON player_claim_grants(player_id)
    WHERE revoked_at IS NULL AND claimed_at IS NULL;
CREATE INDEX player_claim_grants_roster ON player_claim_grants(tournament_id, player_id, created_at DESC, id DESC);

-- Ordinary joining remains closed. The only exception attaches an account to an
-- existing active entrant using a grant consumed by this very transaction.
CREATE OR REPLACE FUNCTION require_joinable_tournament() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE parent_status tournament_status;
BEGIN
    SELECT status INTO parent_status FROM tournaments WHERE id = NEW.tournament_id FOR SHARE;
    IF parent_status IN ('completed', 'archived') THEN
        IF TG_TABLE_NAME = 'tournament_memberships' AND TG_OP = 'INSERT' THEN
            IF NEW.role = 'player' AND EXISTS (
                SELECT 1 FROM player_claim_grants g
                JOIN users u ON u.id = g.claimed_by_user_id AND u.player_id = g.player_id
                JOIN tournament_players tp ON tp.tournament_id = g.tournament_id AND tp.player_id = g.player_id
                JOIN players p ON p.id = g.player_id
                WHERE g.tournament_id = NEW.tournament_id AND g.claimed_by_user_id = NEW.user_id
                  AND g.claimed_transaction = pg_current_xact_id() AND g.claimed_at IS NOT NULL
                  AND g.revoked_at IS NULL AND g.expires_at > clock_timestamp()
                  AND tp.status = 'active' AND p.active
            ) THEN RETURN NEW; END IF;
        END IF;
        RAISE EXCEPTION 'the tournament is closed to new invitations and members'
            USING ERRCODE = '23514', CONSTRAINT = 'tournament_closed_to_joining';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TABLE tournament_player_withdrawals (
    tournament_id UUID NOT NULL,
    player_id UUID NOT NULL,
    withdrawn_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    withdrawn_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (tournament_id, player_id),
    FOREIGN KEY (tournament_id, player_id) REFERENCES tournament_players(tournament_id, player_id) ON DELETE RESTRICT
);
CREATE FUNCTION guard_player_withdrawal() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE actor UUID; session UUID; parent_status tournament_status;
BEGIN
    IF OLD.status = NEW.status THEN RETURN NEW; END IF;
    IF NEW.status <> 'withdrawn' THEN
        RAISE EXCEPTION 'withdrawal cannot be reversed' USING ERRCODE='23514', CONSTRAINT='player_withdrawn';
    END IF;
    -- HTTP locks rounds before parent/entrant; direct reverse-order writes fail.
    PERFORM id FROM rounds WHERE tournament_id=OLD.tournament_id ORDER BY id FOR UPDATE NOWAIT;
    SELECT status INTO parent_status FROM tournaments WHERE id=OLD.tournament_id FOR SHARE;
    IF parent_status IN ('completed','archived') THEN
        RAISE EXCEPTION 'tournament is closed' USING ERRCODE='23514', CONSTRAINT='tournament_closed_to_joining';
    END IF;
    session := nullif(current_setting('app.player_withdrawal_session',true),'')::UUID;
    SELECT s.user_id INTO actor FROM user_sessions s JOIN users u ON u.id=s.user_id
    JOIN tournament_memberships m ON m.user_id=u.id AND m.tournament_id=OLD.tournament_id
    WHERE s.id=session AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp()
      AND s.credential_generation=u.credential_generation AND m.role='admin' FOR SHARE OF s,u,m;
    IF actor IS NULL THEN
        RAISE EXCEPTION 'administrator required' USING ERRCODE='23514', CONSTRAINT='player_withdrawal_admin_required';
    END IF;
    PERFORM u.id FROM users u WHERE u.player_id=OLD.player_id FOR SHARE NOWAIT;
    PERFORM m.user_id FROM tournament_memberships m JOIN users u ON u.id=m.user_id
        WHERE u.player_id=OLD.player_id AND m.tournament_id=OLD.tournament_id FOR SHARE OF m NOWAIT;
    IF EXISTS(SELECT 1 FROM users u LEFT JOIN tournament_memberships m ON m.user_id=u.id AND m.tournament_id=OLD.tournament_id
        WHERE u.player_id=OLD.player_id AND (u.id=actor OR m.role='admin')) THEN
        RAISE EXCEPTION 'administrator cannot be withdrawn' USING ERRCODE='23514', CONSTRAINT='player_is_admin';
    END IF;
    IF EXISTS(SELECT 1 FROM rounds r WHERE r.tournament_id=OLD.tournament_id AND r.status='draft' AND (
        EXISTS(SELECT 1 FROM team_memberships m WHERE m.round_id=r.id AND m.player_id=OLD.player_id)
        OR EXISTS(SELECT 1 FROM flight_memberships m WHERE m.round_id=r.id AND m.player_id=OLD.player_id)
        OR EXISTS(SELECT 1 FROM singles_match_opponents m WHERE m.round_id=r.id AND m.player_id=OLD.player_id))) THEN
        RAISE EXCEPTION 'remove draft assignments first' USING ERRCODE='23514', CONSTRAINT='player_assigned_draft';
    END IF;
    IF EXISTS(SELECT 1 FROM rounds r JOIN round_handicap_snapshots s ON s.round_id=r.id
        WHERE r.tournament_id=OLD.tournament_id AND s.player_id=OLD.player_id AND r.status IN ('open','completed')) THEN
        RAISE EXCEPTION 'lock participating rounds first' USING ERRCODE='23514', CONSTRAINT='player_round_in_progress';
    END IF;
    INSERT INTO tournament_player_withdrawals(tournament_id,player_id,withdrawn_by_user_id) VALUES(OLD.tournament_id,OLD.player_id,actor);
    UPDATE player_claim_grants SET revoked_at=clock_timestamp() WHERE player_id=OLD.player_id AND claimed_at IS NULL AND revoked_at IS NULL;
    RETURN NEW;
END;
$$;
CREATE TRIGGER tournament_players_guard_withdrawal BEFORE UPDATE OF status ON tournament_players
FOR EACH ROW EXECUTE FUNCTION guard_player_withdrawal();
CREATE FUNCTION protect_player_withdrawal_record() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='INSERT' AND pg_trigger_depth()=2 THEN RETURN NEW; END IF;
    RAISE EXCEPTION 'withdrawal records are append-only' USING ERRCODE='23514', CONSTRAINT='player_withdrawal_record_immutable';
END;
$$;
CREATE TRIGGER tournament_player_withdrawals_protect BEFORE INSERT OR UPDATE OR DELETE ON tournament_player_withdrawals
FOR EACH ROW EXECUTE FUNCTION protect_player_withdrawal_record();
