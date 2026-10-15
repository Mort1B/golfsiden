CREATE FUNCTION test_configuration_wait() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.status<>'draft' OR NEW.updated_at<=OLD.updated_at OR
    NEW.course_id IS NOT DISTINCT FROM OLD.course_id OR
    NEW.tee_id IS NOT DISTINCT FROM OLD.tee_id OR
    NEW.course_name<>'Exact Course' OR NEW.tee_name<>'White' OR NEW.number_of_holes<>2 OR
    NOT EXISTS(SELECT 1 FROM courses WHERE id=NEW.course_id AND name='Exact Course'
      AND location='Oslo, Norway' AND source='manual' AND imported_at IS NOT NULL
      AND provider_course_id IS NULL) OR
    (SELECT count(*) FROM tees WHERE course_id=NEW.course_id)<>1 OR
    NOT EXISTS(SELECT 1 FROM tees WHERE id=NEW.tee_id AND course_id=NEW.course_id
      AND name='White' AND category='male' AND number_of_holes=2
      AND course_rating=71.2 AND slope_rating=125) OR
    (SELECT jsonb_agg(jsonb_build_array(hole_number,par,stroke_index,yardage) ORDER BY hole_number)
      FROM holes WHERE tee_id=NEW.tee_id) IS DISTINCT FROM '[[1,4,2,401],[2,3,1,null]]'::jsonb THEN
  RAISE EXCEPTION 'expected complete pending course hierarchy and changed attachment/timestamp before late wait';
 END IF;
 PERFORM pg_advisory_xact_lock(260015,3);
 RETURN NEW;
END $$;
CREATE TRIGGER test_configuration_wait AFTER UPDATE OF course_id ON rounds
 FOR EACH ROW WHEN(NEW.id='84000000-0000-0000-0000-000000000004')
 EXECUTE FUNCTION test_configuration_wait();
