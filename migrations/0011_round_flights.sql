CREATE TABLE flights (
    id UUID PRIMARY KEY,
    round_id UUID NOT NULL,
    tournament_id UUID NOT NULL,
    name TEXT NOT NULL
        CONSTRAINT flights_name_trimmed_nonempty_check
        CHECK (name = btrim(name) AND name <> ''),
    starting_hole SMALLINT
        CONSTRAINT flights_starting_hole_check
        CHECK (starting_hole BETWEEN 1 AND 36),
    tee_time TIME,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT flights_round_tournament_fkey
        FOREIGN KEY (round_id, tournament_id)
        REFERENCES rounds(id, tournament_id) ON DELETE CASCADE,
    CONSTRAINT flights_round_name_unique UNIQUE (round_id, name),
    CONSTRAINT flights_exact_identity_unique
        UNIQUE (id, round_id, tournament_id)
);

CREATE TABLE flight_memberships (
    flight_id UUID NOT NULL,
    round_id UUID NOT NULL,
    tournament_id UUID NOT NULL,
    player_id UUID NOT NULL,
    display_order SMALLINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT flight_memberships_pkey PRIMARY KEY (flight_id, player_id),
    CONSTRAINT flight_memberships_flight_identity_fkey
        FOREIGN KEY (flight_id, round_id, tournament_id)
        REFERENCES flights(id, round_id, tournament_id) ON DELETE CASCADE,
    CONSTRAINT flight_memberships_tournament_entrant_fkey
        FOREIGN KEY (tournament_id, player_id)
        REFERENCES tournament_players(tournament_id, player_id)
        ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT flight_memberships_round_player_unique
        UNIQUE (round_id, player_id),
    CONSTRAINT flight_memberships_exact_identity_unique
        UNIQUE (flight_id, round_id, tournament_id, player_id)
);

CREATE TABLE flight_scorekeepers (
    flight_id UUID PRIMARY KEY,
    round_id UUID NOT NULL,
    tournament_id UUID NOT NULL,
    player_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT flight_scorekeepers_exact_membership_fkey
        FOREIGN KEY (flight_id, round_id, tournament_id, player_id)
        REFERENCES flight_memberships(
            flight_id,
            round_id,
            tournament_id,
            player_id
        ) ON DELETE CASCADE,
    CONSTRAINT flight_scorekeepers_linked_user_fkey
        FOREIGN KEY (player_id) REFERENCES users(player_id) ON DELETE RESTRICT
);

CREATE TRIGGER flights_set_updated_at
BEFORE UPDATE ON flights
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER flights_protect_round_pairing
BEFORE INSERT OR UPDATE OR DELETE ON flights
FOR EACH ROW EXECUTE FUNCTION protect_round_pairing();

CREATE TRIGGER flight_memberships_protect_round_pairing
BEFORE INSERT OR UPDATE OR DELETE ON flight_memberships
FOR EACH ROW EXECUTE FUNCTION protect_round_pairing();

CREATE TRIGGER flight_scorekeepers_protect_round_pairing
BEFORE INSERT OR UPDATE OR DELETE ON flight_scorekeepers
FOR EACH ROW EXECUTE FUNCTION protect_round_pairing();
