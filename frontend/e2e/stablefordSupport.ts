import { expect, type Page } from '@playwright/test'
import { decodeObject } from '../src/api/decoder'
import { decodeAuthSession } from '../src/api/auth'
import { decodeRound, decodeTournament, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeStablefordScoring } from '../src/api/stableford/decoders'
import { expectedFourBall, type FourBallInput } from '../src/api/fourBall/contracts'
import type { Round } from '../src/api/types'
export async function stablefordFixture(page: Page, beforeOpen?: (round: Round, tournamentId: string) => Promise<void>, mixed = false, beforeStablefordOpen?: (tournamentId: string) => Promise<void>) {
  const stamp = `${Date.now()}_${Math.floor(Math.random() * 10000)}`
  const username = `sf_${stamp}`; const password = 'stableford-browser-password'
  const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
  const response = await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username, password }, player: { display_name: 'Spiller med et svært langt navn som fører Stableford uten forbindelse', handicap_index: 18 } },
    tournament: { name: `Stableford ${stamp}`, description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
    rounds: [...(mixed ? [{ round_number: 1, name: 'Slagspill', round_date: day, scoring_format: 'individual_stroke_play' }] : []),
      { round_number: mixed ? 2 : 1, name: 'Stableford med et langt navn', round_date: day, scoring_format: 'individual_stableford' }],
  } })
  expect(response.status()).toBe(201)
  const body = decodeObject(await response.json(), 'onboarding'); const tournament = decodeTournament(body.tournament); const auth = decodeAuthSession(body.session)
  const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${tournament.id}/rounds`)).json(), tournament.id)
  const first = rounds.at(-1); if (!first || !auth.player_id) throw new Error('Missing fixture round/player')
  async function mutate(path: string, data: unknown = {}, method = 'POST') {
    const result = await page.request.fetch(path, { method, data, headers: { 'x-csrf-token': auth.csrf_token } })
    expect(result.ok(), `Fixture mutation ${path}: ${result.status()}`).toBe(true)
    return result
  }
  for (const initial of rounds) {
    const configured = decodeRound(await (await mutate(`/api/rounds/${initial.id}/course-configuration`, {
      expected_round_updated_at: initial.updated_at, selection: { source: 'manual', course_name: 'Stableford testbane', location: null,
        tee: { category: 'male', name: 'Gul', course_rating: 72, slope_rating: 113, holes: Array.from({ length: 18 }, (_, index) => ({ par: 4, stroke_index: index + 1, distance: null })) } },
    }, 'PUT')).json())
    await mutate(`/api/rounds/${initial.id}/pairings`, { expected_round_updated_at: configured.updated_at, teams: [], flights: [{ id: crypto.randomUUID(), name: 'Flight 1', starting_hole: 1, tee_time: null, members: [{ player_id: auth.player_id }] }], legacy_conversions: [] }, 'PUT')
  }
  const round = decodeRound(await (await page.request.get(`/api/rounds/${first.id}`)).json())
  await beforeOpen?.(round, tournament.id)
  const fresh = decodeTournament(await (await page.request.get(`/api/tournaments/${tournament.id}`)).json())
  await mutate(`/api/tournaments/${tournament.id}/start`, { expected_tournament_updated_at: fresh.updated_at })
  if (mixed) {
    const stroke = rounds[0]; if (!stroke) throw new Error('Missing stroke round')
    await mutate(`/api/rounds/${stroke.id}/open`)
    const path = `/api/rounds/${stroke.id}/scorecards/player/${auth.player_id}`
    const card = decodeObject(await (await page.request.get(`${path}/scoring`)).json(), 'card')
    if (!Array.isArray(card.holes)) throw new Error('Missing holes')
    for (const h of card.holes) await mutate(`/api/rounds/${stroke.id}/scores`, { owner: { type: 'player', id: auth.player_id }, hole_id: decodeObject(h, 'hole').hole_id, gross_strokes: 4 }, 'PUT')
    await mutate(`${path}/confirm`); await mutate(`/api/rounds/${stroke.id}/complete`)
  }
  await beforeStablefordOpen?.(tournament.id)
  await mutate(`/api/rounds/${round.id}/open`)
  const owner = { type: 'player' as const, id: auth.player_id }
  const cardPath = `/api/rounds/${round.id}/stableford/scorecards/${owner.id}`
  const read = async () => decodeStablefordScoring(await (await page.request.get(`${cardPath}/scoring`)).json(), round.id, owner.id)
  const card = await read()
  const save = async (number: number, input: FourBallInput) => {
    const latest = await read(); const hole = latest.holes.find(h => h.hole_number === number); if (!hole) throw new Error('Missing fixture hole')
    await mutate(`/api/rounds/${round.id}/stableford/inputs/conditional`, { request_id: crypto.randomUUID(), owner, hole_id: hole.hole_id, input, expected_score: expectedFourBall(hole.score) }, 'PUT')
  }
  const url = (hole = 1, view = 'hole') => `/score?${new URLSearchParams({ tournament: tournament.id, round: round.id, owner_type: owner.type, owner: owner.id, hole: String(hole), view })}`
  return { auth, owner, card, round, tournament, username, password, read, save, mutate, url, cardPath, invitation: body.invitation }
}
