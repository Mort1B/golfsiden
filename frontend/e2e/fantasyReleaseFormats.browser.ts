import { expect, test } from '@playwright/test'
import { decodeStablefordScoring } from '../src/api/stableford/decoders'
import { decodeFourBallScoring } from '../src/api/fourBall/decoders'
import { decodeRound } from '../src/api/tournaments/decoders'
import { decodeListing } from '../src/api/matchPlay/decoders'
import { decodeCard } from '../src/api/matchPlay/cardDecoder'
import type { FourBallInput } from '../src/api/fourBall/contracts'
import type { MatchCommand } from '../src/api/matchPlay/contracts'
import { fantasyLayouts } from './fantasySupport'
import { at, browserEvidence, releaseFixture, selectFour } from './fantasyReleaseSupport'

test.skip(process.env.GOLF_FANTASY_BROWSER !== '1', 'Requires disposable local API and Chrome.')
test('Stableford distinguishes physical aces from net one and preserves uncapped penalties', async ({ page }) => {
  const evidence = browserEvidence(page), f = await releaseFixture(page, ['individual_stableford'], [36, 36, 36, 36])
  await f.configure(0, [0, 1, 2, 3], false, 3)
  await selectFour(f, 0, [0, 1, 2, 3], 0); await f.start()
  const round = at(f.rounds, 0); await f.mutate(`/api/rounds/${round.id}/open`)
  for (const player of f.players) {
    const path = `/api/rounds/${round.id}/stableford/scorecards/${player}`
    const card = decodeStablefordScoring(await (await page.request.get(`${path}/scoring`)).json(), round.id, player)
    expect(card.playing_handicap).toBe(36)
    for (const [i, hole] of card.holes.entries()) {
      // Two received strokes: gross 3 is net 1/eagle; gross 1 is a physical ace.
      const gross = i === 0 ? 3 : i === 1 ? 1 : i === 2 ? 9 : 5
      const input: FourBallInput = i === 3 ? { type: 'no_score' } : { type: 'numeric', gross_strokes: gross }
      await f.mutate(`/api/rounds/${round.id}/stableford/inputs/conditional`, { request_id: crypto.randomUUID(), owner: { type: 'player', id: player }, hole_id: hole.hole_id, input, expected_score: { type: 'absent' } }, 'PUT')
    }
    await f.mutate(`${path}/confirm`)
  }
  await f.mutate(`/api/rounds/${round.id}/complete`)
  const board = await f.board(0)
  for (const g of board.golfers) {
    expect(g.holes[0]).toMatchObject({ category: 'eagle', net_strokes: 1, points: { state: 'settled', total: 3 } })
    expect(g.holes[1]).toMatchObject({ category: 'ace', points: { state: 'settled', total: 10 } })
    expect(g.holes[2]).toMatchObject({ category: 'quadruple_or_worse', points: { state: 'settled', total: -5 } })
    expect(g.holes[3]).toMatchObject({ category: 'pickup', points: { state: 'settled', total: -5 } })
    expect(g.recorded_hole_points).toBe(3); expect(g.placement_points).toBe(10); expect(g.points).toEqual({ state: 'settled', total: 13 })
  }
  expect(board.managers[0]?.points).toEqual({ state: 'settled', total: 65 })
  await page.goto(f.url); await page.getByRole('button', { name: 'Poengtavler', exact: true }).click()
  await expect(page.locator('.fantasy-standings button').first()).toContainText('65 p · avgjort')
  await page.locator('.fantasy-standings button').first().click()
  await expect(page.locator('.fantasy-contributions li').first()).toContainText('Bidrag ×2: 26 p · avgjort')
  await page.getByRole('button', { name: 'Spillerpoeng', exact: true }).click()
  await page.locator('.fantasy-standings button').filter({ hasText: at(f.names, 0) }).click()
  await expect(page.locator('.fantasy-holes li').nth(0)).toContainText('Eagle · netto 1')
  await expect(page.locator('.fantasy-holes li').nth(0)).toContainText('3 p · avgjort')
  await expect(page.locator('.fantasy-holes li').nth(1)).toContainText('Hole-in-one')
  await expect(page.locator('.fantasy-holes li').nth(1)).toContainText('10 p · avgjort')
  await fantasyLayouts(page, 'release-stableford-aces'); await evidence()
})

test('four-ball credits both partners and early match finishes award outcome points only', async ({ page }) => {
  const evidence = browserEvidence(page), f = await releaseFixture(page, ['four_ball_stroke_play', 'singles_match_play'], [0, 0, 0, 0])
  const teams = await f.configure(0, [0, 1, 2, 3], true)
  await f.configure(1, [0, 1, 2, 3], false)
  const matchRound = at(f.rounds, 1), fresh = decodeRound(await (await page.request.get(`/api/rounds/${matchRound.id}`)).json())
  const matches = decodeListing(await (await f.mutate(`/api/rounds/${matchRound.id}/match-play/matches`, { expected_round_updated_at: fresh.updated_at, matches: [{ first_player_id: at(f.players, 0), second_player_id: at(f.players, 1) }, { first_player_id: at(f.players, 2), second_player_id: at(f.players, 3) }] }, 'PUT')).json(), matchRound.id)
  await selectFour(f, 0, [0, 1, 2, 3], 1); await f.start()
  const round = at(f.rounds, 0); await f.mutate(`/api/rounds/${round.id}/open`)
  for (const [teamIndex, team] of teams.entries()) {
    const path = `/api/rounds/${round.id}/four-ball/scorecards/${team.id}`
    const card = decodeFourBallScoring(await (await page.request.get(`${path}/scoring`)).json(), round.id, team.id)
    for (const hole of card.holes) for (const player of hole.players) await f.mutate(`/api/rounds/${round.id}/four-ball/inputs/conditional`, {
      request_id: crypto.randomUUID(), owner: { type: 'player', id: player.player_id }, hole_id: hole.hole_id,
      input: { type: 'numeric', gross_strokes: hole.hole_number === 1 && teamIndex === 0 ? 3 : 4 }, expected_score: { type: 'absent' },
    }, 'PUT')
    await f.mutate(`${path}/confirm`)
  }
  await f.mutate(`/api/rounds/${round.id}/complete`)
  const first = await f.board(0)
  expect(first.golfers.filter(g => g.team_id === at(teams, 0).id).map(g => g.points)).toEqual([{ state: 'settled', total: 11 }, { state: 'settled', total: 11 }])
  expect(first.golfers.filter(g => g.team_id === at(teams, 1).id).map(g => g.points)).toEqual([{ state: 'settled', total: 8 }, { state: 'settled', total: 8 }])
  expect(first.managers[0]?.points).toEqual({ state: 'settled', total: 49 })
  await f.mutate(`/api/rounds/${matchRound.id}/open`)
  for (const match of matches.matches) {
    const path = `/api/rounds/${matchRound.id}/match-play/matches/${match.match_id}`
    const command = async (command: MatchCommand) => {
      const card = decodeCard(await (await page.request.get(`${path}/scoring`)).json(), matchRound.id, match.match_id, true)
      await f.mutate(`${path}/commands`, { request_id: crypto.randomUUID(), expected_revision: card.revision, command })
    }
    if (match.opponents[0].player_id === at(f.players, 0)) {
      expect(match.opponents[1].player_id).toBe(at(f.players, 1))
      await command({ type: 'report', event: { type: 'hole', hole_number: 1, outcome: 'first', basis: { type: 'numeric', first_gross: 1, second_gross: 4, agreed: true } } })
      await command({ type: 'report', event: { type: 'concession', conceding_player_id: at(f.players, 1), after_hole: 1, communicated: true } })
    } else {
      expect(match.opponents.map(opponent => opponent.player_id)).toEqual([at(f.players, 2), at(f.players, 3)])
      for (let h = 1; h <= 18; h++) await command({ type: 'report', event: { type: 'hole', hole_number: h, outcome: 'halved', basis: { type: 'agreed_halve', play_begun: true, mutual_agreement: true } } })
    }
    await command({ type: 'confirm', result_agreed_or_awarded: true })
  }
  await f.mutate(`/api/rounds/${matchRound.id}/complete`)
  const match = await f.board(1)
  for (const [i, expected] of [3, -1, 1, 1].entries()) {
    const golfer = match.golfers.find(g => g.player_id === at(f.players, i))
    expect(golfer?.points).toEqual({ state: 'settled', total: expected }); expect(golfer?.holes).toEqual([]); expect(golfer?.placement_points).toBeNull()
  }
  expect(match.managers[0]?.points).toEqual({ state: 'settled', total: 3 }); expect(match.managers[0]?.lineup?.origin).toBe('carried_forward')
  expect((await f.totals()).managers[0]?.points).toEqual({ state: 'settled', total: 52 })
  await page.goto(f.url); await page.getByRole('button', { name: 'Poengtavler', exact: true }).click()
  await expect(page.locator('.fantasy-standings button').first()).toContainText('52 p · avgjort')
  await page.locator('.fantasy-standings button').first().click()
  await expect(page.locator('.fantasy-contributions li').nth(5)).toContainText('Bidrag ×2: -2 p · avgjort')
  await page.getByRole('button', { name: 'Spillerpoeng', exact: true }).click()
  await page.locator('.fantasy-standings button').filter({ hasText: at(f.names, 0) }).click()
  await expect(page.locator('.fantasy-detail')).toContainText('Matchresultat: Seier +3. Ingen hull- eller plasseringspoeng.')
  await expect(page.locator('.fantasy-holes li')).toHaveCount(18)
  await fantasyLayouts(page, 'release-fourball-match'); await evidence()
})
