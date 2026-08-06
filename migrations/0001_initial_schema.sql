CREATE TYPE user_role AS ENUM ('admin', 'scorer', 'player', 'viewer');
CREATE TYPE tournament_status AS ENUM ('draft', 'active', 'completed', 'archived');
CREATE TYPE scoring_mode AS ENUM ('individual', 'team', 'combined');
CREATE TYPE participant_status AS ENUM ('active', 'withdrawn');
CREATE TYPE round_status AS ENUM ('draft', 'open', 'completed', 'locked');
CREATE TYPE scoring_format AS ENUM ('individual_stroke_play', 'team_scramble');

CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    role user_role NOT NULL DEFAULT 'viewer',
    password_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE players (
    id UUID PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (btrim(display_name) <> ''),
    current_handicap_index NUMERIC(4,1) NOT NULL CHECK (current_handicap_index BETWEEN -10 AND 54),
    email TEXT UNIQUE,
    profile_image_ref TEXT,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE handicap_history (
    id UUID PRIMARY KEY,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    handicap_index NUMERIC(4,1) NOT NULL CHECK (handicap_index BETWEEN -10 AND 54),
    effective_from TIMESTAMPTZ NOT NULL DEFAULT now(),
    changed_by UUID REFERENCES users(id),
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX handicap_history_player_time_idx ON handicap_history(player_id, effective_from DESC);

CREATE TABLE tournaments (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    description TEXT NOT NULL DEFAULT '',
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    number_of_rounds SMALLINT NOT NULL CHECK (number_of_rounds BETWEEN 1 AND 30),
    status tournament_status NOT NULL DEFAULT 'draft',
    scoring_mode scoring_mode NOT NULL DEFAULT 'combined',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (end_date >= start_date)
);

CREATE TABLE tournament_players (
    tournament_id UUID NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id),
    tournament_handicap NUMERIC(4,1) NOT NULL CHECK (tournament_handicap BETWEEN -10 AND 54),
    seed SMALLINT,
    status participant_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tournament_id, player_id)
);

CREATE TABLE courses (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    location TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE tees (
    id UUID PRIMARY KEY,
    course_id UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    slope_rating SMALLINT CHECK (slope_rating BETWEEN 55 AND 155),
    course_rating NUMERIC(4,1),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (course_id, name)
);

CREATE TABLE holes (
    id UUID PRIMARY KEY,
    tee_id UUID NOT NULL REFERENCES tees(id) ON DELETE CASCADE,
    hole_number SMALLINT NOT NULL CHECK (hole_number BETWEEN 1 AND 36),
    par SMALLINT NOT NULL CHECK (par BETWEEN 3 AND 6),
    stroke_index SMALLINT NOT NULL CHECK (stroke_index BETWEEN 1 AND 36),
    yardage SMALLINT CHECK (yardage > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tee_id, hole_number),
    UNIQUE (tee_id, stroke_index),
    UNIQUE (id, tee_id)
);

CREATE TABLE rounds (
    id UUID PRIMARY KEY,
    tournament_id UUID NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    round_number SMALLINT NOT NULL CHECK (round_number > 0),
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    round_date DATE NOT NULL,
    course_id UUID REFERENCES courses(id),
    course_name TEXT NOT NULL,
    tee_id UUID REFERENCES tees(id),
    tee_name TEXT NOT NULL,
    number_of_holes SMALLINT NOT NULL DEFAULT 18 CHECK (number_of_holes BETWEEN 1 AND 36),
    status round_status NOT NULL DEFAULT 'draft',
    handicap_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    handicap_allowance_percent SMALLINT NOT NULL DEFAULT 100 CHECK (handicap_allowance_percent BETWEEN 0 AND 100),
    scoring_format scoring_format NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tournament_id, round_number),
    UNIQUE (id, tournament_id)
);

CREATE TABLE round_handicap_snapshots (
    round_id UUID NOT NULL,
    tournament_id UUID NOT NULL,
    player_id UUID NOT NULL,
    handicap_index NUMERIC(4,1) NOT NULL CHECK (handicap_index BETWEEN -10 AND 54),
    course_handicap SMALLINT NOT NULL,
    playing_handicap SMALLINT NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (round_id, player_id),
    FOREIGN KEY (round_id, tournament_id) REFERENCES rounds(id, tournament_id) ON DELETE CASCADE,
    FOREIGN KEY (tournament_id, player_id) REFERENCES tournament_players(tournament_id, player_id)
);

CREATE TABLE teams (
    id UUID PRIMARY KEY,
    round_id UUID NOT NULL,
    tournament_id UUID NOT NULL,
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    starting_hole SMALLINT CHECK (starting_hole BETWEEN 1 AND 36),
    tee_time TIME,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (round_id, tournament_id) REFERENCES rounds(id, tournament_id) ON DELETE CASCADE,
    UNIQUE (round_id, name),
    UNIQUE (id, round_id, tournament_id)
);

CREATE TABLE team_memberships (
    team_id UUID NOT NULL,
    round_id UUID NOT NULL,
    tournament_id UUID NOT NULL,
    player_id UUID NOT NULL,
    display_order SMALLINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, player_id),
    FOREIGN KEY (team_id, round_id, tournament_id) REFERENCES teams(id, round_id, tournament_id) ON DELETE CASCADE,
    FOREIGN KEY (tournament_id, player_id) REFERENCES tournament_players(tournament_id, player_id),
    UNIQUE (round_id, player_id)
);

CREATE TABLE scores (
    id UUID PRIMARY KEY,
    round_id UUID NOT NULL,
    tournament_id UUID NOT NULL,
    hole_id UUID NOT NULL REFERENCES holes(id),
    player_id UUID,
    team_id UUID,
    gross_strokes SMALLINT NOT NULL CHECK (gross_strokes BETWEEN 1 AND 20),
    submitted_by UUID NOT NULL REFERENCES users(id),
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    locked BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (round_id, tournament_id) REFERENCES rounds(id, tournament_id) ON DELETE CASCADE,
    FOREIGN KEY (tournament_id, player_id) REFERENCES tournament_players(tournament_id, player_id),
    FOREIGN KEY (team_id, round_id, tournament_id) REFERENCES teams(id, round_id, tournament_id),
    CHECK ((player_id IS NOT NULL)::integer + (team_id IS NOT NULL)::integer = 1)
);
CREATE UNIQUE INDEX scores_player_hole_idx ON scores(round_id, hole_id, player_id) WHERE player_id IS NOT NULL;
CREATE UNIQUE INDEX scores_team_hole_idx ON scores(round_id, hole_id, team_id) WHERE team_id IS NOT NULL;

CREATE TABLE score_audits (
    id UUID PRIMARY KEY,
    score_id UUID NOT NULL REFERENCES scores(id),
    changed_by UUID NOT NULL REFERENCES users(id),
    old_gross_strokes SMALLINT,
    new_gross_strokes SMALLINT NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE FUNCTION set_updated_at() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

CREATE TRIGGER players_set_updated_at BEFORE UPDATE ON players FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER tournaments_set_updated_at BEFORE UPDATE ON tournaments FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER tournament_players_set_updated_at BEFORE UPDATE ON tournament_players FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER courses_set_updated_at BEFORE UPDATE ON courses FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER tees_set_updated_at BEFORE UPDATE ON tees FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER holes_set_updated_at BEFORE UPDATE ON holes FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER rounds_set_updated_at BEFORE UPDATE ON rounds FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER teams_set_updated_at BEFORE UPDATE ON teams FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER scores_set_updated_at BEFORE UPDATE ON scores FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE FUNCTION protect_locked_round_score() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    target_round UUID;
    target_status round_status;
BEGIN
    target_round = CASE WHEN TG_OP = 'DELETE' THEN OLD.round_id ELSE NEW.round_id END;
    SELECT status INTO target_status FROM rounds WHERE id = target_round;
    IF target_status = 'locked' AND current_setting('app.admin_correction', true) IS DISTINCT FROM 'true' THEN
        RAISE EXCEPTION 'scores in a locked round require the admin correction workflow' USING ERRCODE = '23514';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;
CREATE TRIGGER scores_protect_locked_round BEFORE INSERT OR UPDATE OR DELETE ON scores
FOR EACH ROW EXECUTE FUNCTION protect_locked_round_score();

CREATE FUNCTION audit_score_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' OR NEW.gross_strokes IS DISTINCT FROM OLD.gross_strokes THEN
        INSERT INTO score_audits (id, score_id, changed_by, old_gross_strokes, new_gross_strokes)
        VALUES (gen_random_uuid(), NEW.id, NEW.submitted_by,
                CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE OLD.gross_strokes END,
                NEW.gross_strokes);
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER scores_audit_change AFTER INSERT OR UPDATE ON scores FOR EACH ROW EXECUTE FUNCTION audit_score_change();
