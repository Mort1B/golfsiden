BEGIN;

-- Existing historical scores start at revision 1 without rewriting their scores,
-- timestamps, audits, confirmations, or owners. Every real future score change
-- advances the database-owned revision, including legacy and correction writes.
ALTER TABLE scores ADD COLUMN revision BIGINT NOT NULL DEFAULT 1 CHECK(revision > 0);
CREATE FUNCTION advance_score_revision() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='INSERT' THEN
        IF NEW.revision <> 1 THEN
            RAISE EXCEPTION 'new score revision must be one' USING ERRCODE='23514',CONSTRAINT='score_revision_managed';
        END IF;
    ELSE
        IF NEW.revision IS DISTINCT FROM OLD.revision THEN
            RAISE EXCEPTION 'score revisions are database managed' USING ERRCODE='23514',CONSTRAINT='score_revision_managed';
        END IF;
        IF NEW.gross_strokes IS DISTINCT FROM OLD.gross_strokes THEN
            IF OLD.revision=9223372036854775807 THEN
                RAISE EXCEPTION 'score revision exhausted' USING ERRCODE='23514',CONSTRAINT='score_revision_exhausted';
            END IF;
            NEW.revision=OLD.revision+1;
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER scores_advance_revision BEFORE INSERT OR UPDATE ON scores
FOR EACH ROW EXECUTE FUNCTION advance_score_revision();

-- A receipt is an acknowledgement of a past application, never a current card.
-- Account and round deletion intentionally remove their delivery receipts.
-- Applied score identity is preserved for the lifetime of that score.
CREATE TABLE score_mutation_receipts (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id UUID NOT NULL,
    round_id UUID NOT NULL REFERENCES rounds(id) ON DELETE CASCADE,
    request_hash BYTEA NOT NULL CHECK(octet_length(request_hash)=32),
    applied_score_id UUID NOT NULL REFERENCES scores(id) ON DELETE CASCADE,
    applied_revision BIGINT NOT NULL CHECK(applied_revision>0),
    applied_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(user_id,request_id)
);
CREATE INDEX score_mutation_receipts_round_idx ON score_mutation_receipts(round_id);
CREATE INDEX score_mutation_receipts_score_idx ON score_mutation_receipts(applied_score_id);
CREATE FUNCTION guard_score_mutation_receipt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='INSERT' THEN
        IF current_setting('app.conditional_score_user_id',true) IS DISTINCT FROM NEW.user_id::TEXT
           OR current_setting('app.conditional_score_request_id',true) IS DISTINCT FROM NEW.request_id::TEXT
           OR current_setting('app.score_mutation_round_id',true) IS DISTINCT FROM NEW.round_id::TEXT
           OR NOT EXISTS(SELECT 1 FROM scores WHERE id=NEW.applied_score_id AND round_id=NEW.round_id AND revision=NEW.applied_revision) THEN
            RAISE EXCEPTION 'score receipts require the conditional scoring workflow' USING ERRCODE='23514',CONSTRAINT='score_receipt_context_required';
        END IF;
        RETURN NEW;
    END IF;
    IF TG_OP='UPDATE' OR (
        EXISTS(SELECT 1 FROM users WHERE id=OLD.user_id)
        AND EXISTS(SELECT 1 FROM rounds WHERE id=OLD.round_id)
        AND EXISTS(SELECT 1 FROM scores WHERE id=OLD.applied_score_id)
    ) THEN
        RAISE EXCEPTION 'score receipts are immutable' USING ERRCODE='23514',CONSTRAINT='score_receipt_immutable';
    END IF;
    RETURN OLD;
END;
$$;
CREATE TRIGGER score_mutation_receipts_guard BEFORE INSERT OR UPDATE OR DELETE ON score_mutation_receipts
FOR EACH ROW EXECUTE FUNCTION guard_score_mutation_receipt();
COMMIT;
