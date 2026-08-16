CREATE TYPE tournament_role AS ENUM ('admin', 'scorer', 'player', 'viewer');

CREATE TABLE tournament_memberships (
    tournament_id UUID NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role tournament_role NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tournament_id, user_id)
);

CREATE INDEX tournament_memberships_user_tournament_idx
    ON tournament_memberships (user_id, tournament_id);

CREATE TRIGGER tournament_memberships_set_updated_at
BEFORE UPDATE ON tournament_memberships
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Preserve the authority that global administrators and scorers had before
-- tournament-scoped roles existed. This is a one-time compatibility backfill;
-- runtime authorization does not treat users.role as a cross-tournament bypass.
INSERT INTO tournament_memberships (tournament_id, user_id, role)
SELECT t.id, u.id, u.role::text::tournament_role
FROM tournaments t
CROSS JOIN users u
WHERE u.role IN ('admin', 'scorer')
ON CONFLICT (tournament_id, user_id) DO NOTHING;

-- A credential-linked player receives participation authority only for trips
-- where that exact player profile is already entered. Email is never used.
INSERT INTO tournament_memberships (tournament_id, user_id, role)
SELECT tp.tournament_id, u.id, 'player'
FROM tournament_players tp
JOIN users u ON u.player_id = tp.player_id
ON CONFLICT (tournament_id, user_id) DO NOTHING;

CREATE TABLE tournament_handicap_history (
    id UUID PRIMARY KEY,
    tournament_id UUID NOT NULL,
    player_id UUID NOT NULL,
    handicap_index NUMERIC(4,1) NOT NULL
        CHECK (handicap_index BETWEEN -10 AND 54),
    effective_from TIMESTAMPTZ NOT NULL DEFAULT now(),
    changed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (tournament_id, player_id)
        REFERENCES tournament_players(tournament_id, player_id)
        ON DELETE CASCADE
);

CREATE INDEX tournament_handicap_history_player_time_idx
    ON tournament_handicap_history
       (tournament_id, player_id, effective_from DESC, id DESC);

-- tournament_players.tournament_handicap is authoritative at upgrade time.
-- One history row records that exact starting point for every existing entrant.
INSERT INTO tournament_handicap_history (
    id,
    tournament_id,
    player_id,
    handicap_index,
    effective_from,
    reason,
    created_at
)
SELECT gen_random_uuid(),
       tp.tournament_id,
       tp.player_id,
       tp.tournament_handicap,
       tp.created_at,
       'membership migration backfill',
       tp.created_at
FROM tournament_players tp;
