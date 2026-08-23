CREATE TYPE course_revision_source AS ENUM ('golf_course_api', 'manual');
CREATE TYPE tee_category AS ENUM ('female', 'male');

ALTER TABLE courses
    ADD COLUMN source course_revision_source,
    ADD COLUMN provider_course_id TEXT,
    ADD COLUMN imported_at TIMESTAMPTZ,
    ADD CONSTRAINT courses_revision_provenance_check CHECK (
        (source IS NULL AND provider_course_id IS NULL AND imported_at IS NULL)
        OR (source = 'manual' AND provider_course_id IS NULL AND imported_at IS NOT NULL)
        OR (source = 'golf_course_api'
            AND provider_course_id IS NOT NULL
            AND btrim(provider_course_id) <> ''
            AND octet_length(provider_course_id) <= 200
            AND imported_at IS NOT NULL)
    ),
    ADD CONSTRAINT courses_revision_name_check
        CHECK (octet_length(name) <= 300) NOT VALID,
    ADD CONSTRAINT courses_revision_location_check
        CHECK (location IS NULL OR (btrim(location) <> '' AND octet_length(location) <= 500))
        NOT VALID;

ALTER TABLE tees
    ADD COLUMN category tee_category,
    ADD COLUMN number_of_holes SMALLINT,
    ADD CONSTRAINT tees_revision_fields_check CHECK (
        (category IS NULL AND number_of_holes IS NULL)
        OR (category IS NOT NULL AND number_of_holes BETWEEN 1 AND 36)
    ),
    ADD CONSTRAINT tees_revision_name_check
        CHECK (octet_length(name) <= 100) NOT VALID,
    ADD CONSTRAINT tees_course_rating_check
        CHECK (course_rating IS NULL OR course_rating BETWEEN 1.0 AND 100.0) NOT VALID;

ALTER TABLE holes
    DROP CONSTRAINT holes_par_check,
    ADD CONSTRAINT holes_par_check CHECK (par BETWEEN 2 AND 7);

CREATE FUNCTION protect_finalized_course_revision() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    old_course_id UUID;
    new_course_id UUID;
    parent_source course_revision_source;
    finalized BOOLEAN;
BEGIN
    IF TG_TABLE_NAME = 'courses' THEN
        IF TG_OP <> 'INSERT' AND OLD.source IS NOT NULL THEN
            RAISE EXCEPTION 'finalized course revisions are immutable'
                USING ERRCODE = '23514', CONSTRAINT = 'course_revision_immutable';
        END IF;
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;

    IF TG_TABLE_NAME = 'tees' THEN
        old_course_id = CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE OLD.course_id END;
        new_course_id = CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE NEW.course_id END;
    ELSE
        IF TG_OP <> 'INSERT' THEN
            SELECT course_id INTO old_course_id FROM tees WHERE id = OLD.tee_id;
        END IF;
        IF TG_OP <> 'DELETE' THEN
            SELECT course_id INTO new_course_id FROM tees WHERE id = NEW.tee_id;
        END IF;
    END IF;

    finalized = FALSE;
    FOR parent_source IN
        SELECT source
        FROM courses
        WHERE id = old_course_id OR id = new_course_id
        ORDER BY id
        FOR UPDATE
    LOOP
        finalized = finalized OR parent_source IS NOT NULL;
    END LOOP;

    IF finalized THEN
        RAISE EXCEPTION 'finalized course revisions are immutable'
            USING ERRCODE = '23514', CONSTRAINT = 'course_revision_immutable';
    END IF;

    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

CREATE TRIGGER courses_protect_finalized_revision
BEFORE UPDATE OR DELETE ON courses
FOR EACH ROW EXECUTE FUNCTION protect_finalized_course_revision();

CREATE TRIGGER tees_protect_finalized_revision
BEFORE INSERT OR UPDATE OR DELETE ON tees
FOR EACH ROW EXECUTE FUNCTION protect_finalized_course_revision();

CREATE TRIGGER holes_protect_finalized_revision
BEFORE INSERT OR UPDATE OR DELETE ON holes
FOR EACH ROW EXECUTE FUNCTION protect_finalized_course_revision();

CREATE FUNCTION validate_finalized_course_revision() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    selected_tee_id UUID;
    expected_holes SMALLINT;
    selected_tee_count BIGINT;
    complete_tee_count BIGINT;
    actual_holes BIGINT;
    distinct_stroke_indexes BIGINT;
    minimum_hole SMALLINT;
    maximum_hole SMALLINT;
    minimum_stroke_index SMALLINT;
    maximum_stroke_index SMALLINT;
BEGIN
    IF NEW.source IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT count(*), count(*) FILTER (
        WHERE category IS NOT NULL
          AND number_of_holes IS NOT NULL
          AND slope_rating IS NOT NULL
          AND course_rating IS NOT NULL
    )
    INTO selected_tee_count, complete_tee_count
    FROM tees
    WHERE course_id = NEW.id;

    IF selected_tee_count <> 1 OR complete_tee_count <> 1 THEN
        RAISE EXCEPTION 'a finalized course revision requires exactly one complete tee'
            USING ERRCODE = '23514', CONSTRAINT = 'course_revision_single_complete_tee';
    END IF;

    SELECT id, number_of_holes INTO selected_tee_id, expected_holes
    FROM tees
    WHERE course_id = NEW.id;

    SELECT count(*), count(DISTINCT stroke_index), min(hole_number), max(hole_number),
           min(stroke_index), max(stroke_index)
    INTO actual_holes, distinct_stroke_indexes, minimum_hole, maximum_hole,
         minimum_stroke_index, maximum_stroke_index
    FROM holes
    WHERE tee_id = selected_tee_id;

    IF actual_holes <> expected_holes
       OR distinct_stroke_indexes <> expected_holes
       OR minimum_hole <> 1 OR maximum_hole <> expected_holes
       OR minimum_stroke_index <> 1 OR maximum_stroke_index <> expected_holes THEN
        RAISE EXCEPTION 'a finalized course revision requires complete ordered holes and stroke indexes'
            USING ERRCODE = '23514', CONSTRAINT = 'course_revision_holes_incomplete';
    END IF;

    RETURN NEW;
END;
$$;

CREATE CONSTRAINT TRIGGER courses_validate_finalized_revision
AFTER INSERT OR UPDATE OF source, provider_course_id, imported_at ON courses
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION validate_finalized_course_revision();
