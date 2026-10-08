WITH owner_players AS (
 SELECT player_id FROM team_memberships WHERE round_id=$1 AND team_id=$3 AND $2='team'
 UNION ALL SELECT $3::uuid WHERE $2='player'
), match_facts AS (
 SELECT m.* FROM singles_matches m WHERE m.round_id=$1 AND $2='player' AND (m.first_player_id=$3 OR m.second_player_id=$3)
)
SELECT jsonb_build_object(
 'version',1,'round',to_jsonb(r),'owner_kind',$2::text,'owner_id',$3::uuid,
 'generation',COALESCE((SELECT generation FROM fantasy_owner_generations WHERE round_id=$1 AND owner_kind=$2 AND owner_id=$3),0),
 'holes',(SELECT COALESCE(jsonb_agg(to_jsonb(h) ORDER BY h.hole_number),'[]'::jsonb) FROM holes h WHERE h.tee_id=r.tee_id),
 'tee',(SELECT to_jsonb(t) FROM tees t WHERE t.id=r.tee_id),
 'players',(SELECT COALESCE(jsonb_agg(player_id ORDER BY player_id),'[]'::jsonb) FROM owner_players),
 'snapshots',(SELECT COALESCE(jsonb_agg(to_jsonb(s) ORDER BY s.player_id),'[]'::jsonb) FROM round_handicap_snapshots s WHERE s.round_id=$1 AND (s.player_id IN (SELECT player_id FROM owner_players) OR s.player_id IN (SELECT first_player_id FROM match_facts UNION SELECT second_player_id FROM match_facts))),
 'team_snapshot',(SELECT to_jsonb(s) FROM round_team_handicap_snapshots s WHERE s.round_id=$1 AND s.team_id=$3 AND $2='team'),
 'scores',(SELECT COALESCE(jsonb_agg(to_jsonb(s) ORDER BY s.hole_id,s.id),'[]'::jsonb) FROM scores s WHERE s.round_id=$1 AND (($2='player' AND s.player_id=$3) OR ($2='team' AND s.team_id=$3))),
 'four_ball',(SELECT COALESCE(jsonb_agg(to_jsonb(s) ORDER BY s.player_id,s.hole_id,s.id),'[]'::jsonb) FROM four_ball_inputs s WHERE s.round_id=$1 AND s.player_id IN (SELECT player_id FROM owner_players)),
 'stableford',(SELECT COALESCE(jsonb_agg(to_jsonb(s) ORDER BY s.hole_id,s.id),'[]'::jsonb) FROM stableford_inputs s WHERE s.round_id=$1 AND s.player_id IN (SELECT player_id FROM owner_players)),
 'confirmation',(SELECT COALESCE(jsonb_agg(to_jsonb(c) ORDER BY c.id),'[]'::jsonb) FROM scorecard_confirmations c WHERE c.round_id=$1 AND (($2='player' AND c.player_id=$3) OR ($2='team' AND c.team_id=$3))),
 'matches',(SELECT COALESCE(jsonb_agg(to_jsonb(m) ORDER BY m.id),'[]'::jsonb) FROM match_facts m),
 'match_notes',(SELECT COALESCE(jsonb_agg(to_jsonb(n) ORDER BY n.player_id,n.hole_number),'[]'::jsonb) FROM singles_match_notes n WHERE n.match_id IN (SELECT id FROM match_facts)),
 'match_audits',(SELECT COALESCE(jsonb_agg(to_jsonb(a) ORDER BY a.revision,a.id),'[]'::jsonb) FROM singles_match_audits a WHERE a.match_id IN (SELECT id FROM match_facts))
) FROM rounds r WHERE r.id=$1
