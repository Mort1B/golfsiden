BEGIN;
-- Retry receipts, not account or scoring history. Deleting the owning user or
-- tournament removes only its receipt; normal HTTP APIs do not delete either.
CREATE TABLE tournament_creation_requests (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id UUID NOT NULL,
    request_hash BYTEA NOT NULL CHECK (octet_length(request_hash) = 32),
    tournament_id UUID NOT NULL UNIQUE REFERENCES tournaments(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, request_id)
);
COMMIT;
