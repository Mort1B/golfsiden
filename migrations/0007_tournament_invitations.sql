CREATE TABLE tournament_invitations (
    id UUID PRIMARY KEY,
    tournament_id UUID NOT NULL
        REFERENCES tournaments(id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE
        CHECK (octet_length(token_hash) = 32),
    created_by_user_id UUID NOT NULL
        REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    max_uses INTEGER,
    FOREIGN KEY (tournament_id, created_by_user_id)
        REFERENCES tournament_memberships(tournament_id, user_id)
        ON DELETE CASCADE,
    CHECK (expires_at > created_at),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at),
    CHECK (max_uses IS NULL OR max_uses > 0)
);

-- Supports deterministic invitation management within one tournament.
CREATE INDEX tournament_invitations_tournament_created_idx
    ON tournament_invitations (tournament_id, created_at DESC, id DESC);
