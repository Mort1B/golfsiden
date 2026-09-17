// Synthetic, public fixture identities only. No backend writes or credentials.
export const id = n => `10000000-0000-4000-8000-${String(n).padStart(12, '0')}`
export const tournamentId = id(1)
export const playerId = id(10000)
const timestamp = '2026-09-17T10:00:00Z'
export function fixture(matchCount, roundCount, longNames = true) {
  const name = n => longNames ? `Spiller ${n} med et svært langt navn fra golfklubben ved fjorden` : `Spiller ${n}`
  const rounds = Array.from({ length: roundCount }, (_, r) => ({
    id: id(100 + r), tournament_id: tournamentId, round_number: r + 1,
    name: `Runde ${r + 1} med lange navn og bekreftede matcher`, round_date: '2026-09-17',
    course_id: id(2), course_name: 'Testbane', tee_id: id(3), tee_name: 'Gul', number_of_holes: 18,
    status: 'open', handicap_enabled: true, handicap_allowance_percent: 100,
    scoring_format: 'singles_match_play', created_at: timestamp, updated_at: timestamp,
  }))
  const listings = new Map(rounds.map((round, r) => {
    const matches = Array.from({ length: matchCount }, (_, m) => {
      const opponents = [0, 1].map(p => ({ player_id: id(10000 + m * 2 + p), display_name: name(m * 2 + p + 1), playing_handicap: 0 }))
      return {
        format: 'singles_match_play', match_id: id(100000 + r * 1000 + m), round_id: round.id,
        tournament_id: tournamentId, round_status: 'open', mode: 'net', opponents,
        relative_handicaps: [0, 0],
        holes: Array.from({ length: 18 }, (_, h) => ({ hole_number: h + 1, par: 4, stroke_index: h + 1 })),
        notes: Array.from({ length: 18 }, (_, h) => opponents.map(p => ({ player_id: p.player_id, hole_number: h + 1, gross_strokes: 4 }))).flat(),
        events: Array.from({ length: 18 }, (_, h) => ({ type: 'hole', hole_number: h + 1, outcome: 'halved', basis: { type: 'numeric', first_gross: 4, second_gross: 4, agreed: true } })),
        resolved_holes: 18, lead: 0, finish: { type: 'draw' }, confirmed: true,
        correction_pending: false, half_points: [1, 1], visibility: { mode: 'full' },
      }
    })
    return [round.id, { round_id: round.id, matches, writable_match_ids: matches.map(m => m.match_id) }]
  }))
  return {
    rounds, listings,
    session: { user_id: id(4), username: 'synthetic_admin', display_name: 'Syntetisk administrator', role: 'admin', player_id: playerId, csrf_token: 'synthetic-unused', expires_at: '2099-01-01T00:00:00Z' },
    table: { tournament_id: tournamentId, entries: Array.from({ length: matchCount * 2 }, (_, p) => ({
      player_id: id(10000 + p), display_name: name(p + 1), half_points: roundCount,
      played: roundCount, wins: 0, draws: roundCount, losses: 0, position: 1,
    })) },
  }
}
