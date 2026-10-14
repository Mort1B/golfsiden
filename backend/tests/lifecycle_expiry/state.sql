SELECT jsonb_build_object(
    'round', (SELECT to_jsonb(r) FROM rounds r WHERE id=$1),
    'tournament', (SELECT to_jsonb(t) FROM tournaments t WHERE id=$2),
    'handicap_locks', (SELECT coalesce(jsonb_agg(to_jsonb(l)), '[]') FROM tournament_handicap_locks l WHERE tournament_id=$2),
    'players', (SELECT coalesce(jsonb_agg(to_jsonb(p) ORDER BY player_id), '[]') FROM round_handicap_snapshots p WHERE round_id=$1),
    'teams', (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY team_id), '[]') FROM round_team_handicap_snapshots t WHERE round_id=$1),
    'scores', (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM scores s WHERE round_id=$1),
    'score_audits', (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY a.id), '[]') FROM score_audits a JOIN scores s ON s.id=a.score_id WHERE s.round_id=$1),
    'confirmations', (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY player_id,team_id), '[]') FROM scorecard_confirmations c WHERE round_id=$1),
    'matches', (SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY id), '[]') FROM singles_matches m WHERE round_id=$1),
    'opponents', (SELECT coalesce(jsonb_agg(to_jsonb(o) ORDER BY player_id), '[]') FROM singles_match_opponents o WHERE round_id=$1),
    'match_notes', (SELECT coalesce(jsonb_agg(to_jsonb(n) ORDER BY match_id,player_id,hole_number), '[]') FROM singles_match_notes n WHERE match_id IN (SELECT id FROM singles_matches WHERE round_id=$1)),
    'match_audits', (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM singles_match_audits a WHERE match_id IN (SELECT id FROM singles_matches WHERE round_id=$1)),
    'match_receipts', (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY user_id,request_id), '[]') FROM singles_match_receipts r WHERE match_id IN (SELECT id FROM singles_matches WHERE round_id=$1))
)
