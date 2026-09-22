import { fixture, playerId, tournamentId } from '../fixtures.mjs'
export { playerId, tournamentId }
export const scenarios = [
  { name: 'typical', matches: 12, rounds: 1, restricted: false, widths: [390] },
  { name: 'populated', matches: 24, rounds: 3, restricted: false, widths: [320, 390, 1280] },
  { name: 'restricted-final', matches: 24, rounds: 3, restricted: true, widths: [390] },
  { name: 'stress', matches: 100, rounds: 3, restricted: false, widths: [390] },
]
export function dataFor(scenario, epoch = 0) {
  const data = fixture(scenario.matches, scenario.rounds)
  // Completed draws with varied numeric evidence, not identical all-four cards.
  for (const [r, listing] of [...data.listings.values()].entries()) for (const [m, card] of listing.matches.entries()) {
    card.events = card.events.map((event, h) => {
      const outcome = ['first', 'second', 'halved'][(m + h) % 3]
      const base = 3 + (m * 7 + r * 3 + h * 11) % 4
      return { ...event, outcome, basis: { type: 'numeric', first_gross: base + Number(outcome === 'second'), second_gross: base + Number(outcome === 'first'), agreed: true } }
    })
    card.notes = card.events.flatMap(e => card.opponents.map((p, i) => ({ player_id: p.player_id, hole_number: e.hole_number, gross_strokes: i === 0 ? e.basis.first_gross : e.basis.second_gross })))
  }
  if (scenario.restricted) {
    data.session.role = 'viewer'
    for (const listing of data.listings.values()) listing.writable_match_ids = []
    const last = data.rounds.at(-1), listing = data.listings.get(last.id)
    listing.matches = listing.matches.map(card => ({ ...card,
      holes: card.holes.slice(0, 9), notes: card.notes.filter(n => n.hole_number <= 9), events: card.events.slice(0, 9),
      resolved_holes: 9, finish: null, confirmed: null, correction_pending: null, half_points: null, visibility: { mode: 'front_nine' },
    }))
    listing.writable_match_ids = []
    for (const entry of data.table.entries) { entry.half_points--; entry.played--; entry.draws-- }
  }
  for (const listing of data.listings.values()) for (const card of listing.matches) for (const opponent of card.opponents) opponent.display_name += ` [epoch ${epoch}]`
  for (const entry of data.table.entries) entry.display_name += ` [epoch ${epoch}]`
  return data
}
export function playerSubset(listing) {
  const matches = listing.matches.filter(card => card.opponents.some(p => p.player_id === playerId))
  return { ...listing, matches, writable_match_ids: listing.writable_match_ids.filter(id => matches.some(m => m.match_id === id)) }
}
