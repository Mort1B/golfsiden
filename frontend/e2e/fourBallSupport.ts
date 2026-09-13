import { expect, type Browser, type Page } from '@playwright/test'
import { decodeObject, decodeString, decodeUuid } from '../src/api/decoder'
import { decodeAuthSession } from '../src/api/auth'
import { decodeRound, decodeTournament, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeFourBallScoring } from '../src/api/fourBall/decoders'
import { expectedFourBall, type FourBallInput } from '../src/api/fourBall/contracts'
import type { Round, Tournament } from '../src/api/types'
export async function fourBallFixture(page: Page, browser: Browser, beforeOpen?: (round: Round, tournament: Tournament) => Promise<void>) {
  const stamp = `${Date.now()}_${Math.floor(Math.random() * 10000)}`
  const username = `fourball_${stamp}`, password = 'fourball-browser-password'
  const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
  const response = await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username, password }, player: { display_name: 'Andreas med et svært langt navn i four-ball-turneringen', handicap_index: 0 } },
    tournament: { name: `Four-ball ${stamp}`, description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
    rounds: [{ round_number: 1, name: 'Four-ball med to partnere', round_date: day, scoring_format: 'four_ball_stroke_play' }],
  } })
  expect(response.status()).toBe(201)
  const body = decodeObject(await response.json(), 'onboarding')
  const tournament = decodeTournament(body.tournament), auth = decodeAuthSession(body.session)
  const invitation = decodeObject(body.invitation, 'invitation')
  const otherContext = await browser.newContext()
  let secondId: string
  try {
    const other = await otherContext.newPage()
    const registered = await other.request.post(`http://127.0.0.1:5173/api/invitations/${decodeUuid(invitation.id, 'id')}/register`, { data: {
      token: decodeString(invitation.token, 'token'), account: { username: `partner_${stamp}`, password },
      player: { display_name: 'Bjørn partner med ekstra slag og et veldig langt navn', handicap_index: 21.2 },
    } })
    expect(registered.status()).toBe(201)
    secondId = decodeUuid(decodeObject(await registered.json(), 'registration').player_id, 'player_id')
  } finally { await otherContext.close() }
  const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${tournament.id}/rounds`)).json(), tournament.id)
  const first = rounds[0]; if (!first || !auth.player_id) throw new Error('Missing fixture round/player')
  async function mutate(path: string, data: unknown = {}, method = 'POST') {
    const result = await page.request.fetch(path, { method, data, headers: { 'x-csrf-token': auth.csrf_token } })
    expect(result.ok(), `Fixture mutation status ${result.status()}`).toBe(true)
    return result
  }
  const round = decodeRound(await (await mutate(`/api/rounds/${first.id}/course-configuration`, {
    expected_round_updated_at: first.updated_at, selection: { source: 'manual', course_name: 'Four-ball testbane', location: null,
      tee: { category: 'male', name: 'Gul', course_rating: 72, slope_rating: 113, holes: Array.from({ length: 18 }, (_, index) => ({ par: 4, stroke_index: index + 1, distance: null })) } },
  }, 'PUT')).json())
  expect(round.handicap_allowance_percent).toBe(85)
  const teamId = crypto.randomUUID()
  const members = [{ player_id: auth.player_id }, { player_id: secondId }]
  await mutate(`/api/rounds/${round.id}/pairings`, { expected_round_updated_at: round.updated_at,
    teams: [{ id: teamId, name: 'Andreas og Bjørns lag med et ekstra langt lagnavn', members, schedule_flight_id: null }],
    flights: [{ id: crypto.randomUUID(), name: 'Flight 1', starting_hole: 1, tee_time: null, members }], legacy_conversions: [] }, 'PUT')
  await beforeOpen?.(round, tournament)
  const fresh = decodeTournament(await (await page.request.get(`/api/tournaments/${tournament.id}`)).json())
  await mutate(`/api/tournaments/${tournament.id}/start`, { expected_tournament_updated_at: fresh.updated_at })
  await mutate(`/api/rounds/${round.id}/open`)
  const cardPath = `/api/rounds/${round.id}/four-ball/scorecards/${teamId}`
  const read = async () => decodeFourBallScoring(await (await page.request.get(`${cardPath}/scoring`)).json(), round.id, teamId)
  const card = await read()
  const save = async (number: number, playerId: string, input: FourBallInput) => {
    const latest = await read(), hole = latest.holes.find(item => item.hole_number === number)
    const player = hole?.players.find(item => item.player_id === playerId)
    if (!hole || !player) throw new Error('Missing fixture player/hole')
    await mutate(`/api/rounds/${round.id}/four-ball/inputs/conditional`, { request_id: crypto.randomUUID(), hole_id: hole.hole_id,
      owner: { type: 'player', id: playerId }, input, expected_score: expectedFourBall(player.score) }, 'PUT')
  }
  const url = (hole = 1, view = 'hole') => `/score?${new URLSearchParams({ tournament: tournament.id, round: round.id, owner_type: 'team', owner: teamId, hole: String(hole), view })}`
  return { auth, card, round, tournament, teamId, secondId, firstId: auth.player_id, username, partnerUsername: `partner_${stamp}`, password, read, save, mutate, url, cardPath }
}
