SELECT jsonb_build_object(
 'round',(SELECT to_jsonb(r) FROM rounds r WHERE id=$1),
 'teams',(SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id),'[]') FROM teams t WHERE round_id=$1),
 'team_members',(SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY team_id,player_id),'[]') FROM team_memberships m WHERE round_id=$1),
 'flights',(SELECT coalesce(jsonb_agg(to_jsonb(f) ORDER BY id),'[]') FROM flights f WHERE round_id=$1),
 'flight_members',(SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY flight_id,player_id),'[]') FROM flight_memberships m WHERE round_id=$1),
 'tournament',(SELECT to_jsonb(t) FROM tournaments t WHERE id=$2),
 'entrants',(SELECT coalesce(jsonb_agg(to_jsonb(p) ORDER BY player_id),'[]') FROM tournament_players p WHERE tournament_id=$2),
 'players',(SELECT coalesce(jsonb_agg(to_jsonb(p) ORDER BY id),'[]') FROM players p WHERE id IN (SELECT player_id FROM tournament_players WHERE tournament_id=$2)),
 'snapshots',(SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY player_id),'[]') FROM round_handicap_snapshots s WHERE round_id=$1),
 'team_snapshots',(SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY team_id),'[]') FROM round_team_handicap_snapshots s WHERE round_id=$1),
 'scores',(SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id),'[]') FROM scores s WHERE round_id=$1),
 'confirmations',(SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY player_id,team_id),'[]') FROM scorecard_confirmations c WHERE round_id=$1)
)
