import { test, expect } from '@playwright/test'
import { writeFile } from 'node:fs/promises'
import { decodeTournamentList, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeObject } from '../src/api/decoder'
import { LeaderboardTestApi } from './leaderboardTestApi'
import { login, mutate, progress, scorecard, tournamentBoard, roundBoard, assertRendered, nextBoard, fillAndConfirm, layout } from './leaderboardLiveSupport'

test.skip(process.env.GOLF_LEADERBOARD_LIVE_BROWSER !== '1', 'Requires fresh disposable seed and GOLF_LEADERBOARD_LIVE_BROWSER=1.')

const server = new LeaderboardTestApi()
test.beforeAll(() => server.start())
test.afterAll(() => server.stop())

test('real saved results, same-account desktop/phone, policy exclusions, reconnect and hidden final', async ({ page, browser }) => {
  const auth = await login(page)
  const api = page.request
  const trip = decodeTournamentList(await (await api.get('/api/tournaments')).json()).find(item => item.name === 'Guttas Golf 2026')
  if (!trip) throw new Error('Fresh seed required')
  const rounds = decodeTournamentRounds(await (await api.get(`/api/tournaments/${trip.id}/rounds`)).json(), trip.id)
  const first = rounds[0]; const individual = rounds[2]; const foursomes = rounds[3]; const final = rounds[4]
  if (!first || !individual || !foursomes || !final) throw new Error('Missing seed rounds')
  const phoneContext = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true })
  // Native SSE connects directly to the owned API: Vite's development proxy can
  // keep its downstream response open after an upstream process crashes.
  await phoneContext.addInitScript(() => {
    const NativeEventSource = window.EventSource
    window.EventSource = class extends NativeEventSource {
      constructor(url: string | URL, init?: EventSourceInit) {
        super(new URL(String(url), 'http://127.0.0.1:3000').href, init)
      }
    }
  })
  const phone = await phoneContext.newPage()
  await login(phone) // Same identity, separate authenticated session, like the reported PC + iPhone setup.
  const failures: string[] = []; const requests: string[] = []
  for (const target of [page, phone]) {
    target.on('pageerror', error => failures.push(error.message))
    target.on('console', msg => { if (msg.type() === 'error' && !msg.text().startsWith('Failed to load resource:')) failures.push(msg.text()) })
  }
  phone.on('request', request => { if (request.method() === 'GET' && request.url().includes('/api/')) requests.push(new URL(request.url()).pathname) })
  let expectedOutage = false
  let expectedSaveFailure = false
  for (const target of [page, phone]) {
    target.on('response', response => {
      const path = new URL(response.url()).pathname
      if (response.status() >= 400 && !expectedOutage
        && !(path === '/api/auth/session' && response.status() === 401)
        && !(expectedSaveFailure && path.endsWith('/scores') && response.status() === 503)) failures.push(`${response.status()} ${path}`)
    })
    target.on('requestfailed', request => {
      if (!expectedOutage && !request.failure()?.errorText.includes('ERR_ABORTED')) failures.push(new URL(request.url()).pathname)
    })
  }
  const measurements: unknown[] = []
  try {
    await mutate(api, `/api/tournaments/${trip.id}/start`, auth.csrf_token, { expected_tournament_updated_at: trip.updated_at })
    for (const round of [first, individual, foursomes]) {
      await mutate(api, `/api/rounds/${round.id}/open`, auth.csrf_token)
      const owner = (await progress(api, round)).owners[0]?.owner
      if (!owner) throw new Error('Missing owner')
      const card = await scorecard(api, round, owner); const hole = card.holes[0]
      if (!hole) throw new Error('Missing hole')
      await phone.goto(`/leaderboard?tournament=${trip.id}&scope=tournament&metric=gross`)
      await assertRendered(phone, await tournamentBoard(phone.request, trip.id, 'gross'))
      await page.goto(`/score?tournament=${trip.id}&round=${round.id}&owner_type=${owner.type}&owner=${owner.id}&hole=1&view=hole`)
      await expect(page.getByRole('button', { name: /Registrer par/ })).toBeEnabled()
      requests.length = 0
      const live = nextBoard(phone, trip.id, 'gross')
      const started = Date.now()
      await page.getByRole('button', { name: /Registrer par/ }).click()
      await expect(page.locator('.score-sync')).toHaveText(/^(Synkronisert|Lagret)$/)
      await live
      const stored = await scorecard(api, round, owner)
      expect(stored.holes[0]?.score?.gross_strokes).toBe(hole.par)
      const board = await tournamentBoard(phone.request, trip.id, 'gross')
      expect(board.current_round_id).toBe(round.id)
      const affected = board.entries.filter(entry => entry.contributions.some(item => item.round_id === round.id && item.owner.id === owner.id))
      expect(affected.length).toBe(owner.type === 'team' ? 2 : 1)
      expect(affected.every(entry => entry.contributions.some(item => item.round_id === round.id && item.provisional && item.counted && item.holes_scored === 1))).toBe(true)
      await assertRendered(phone, board)
      const roundResult = await roundBoard(phone.request, round, 'gross')
      expect(roundResult.entries.find(entry => entry.owner.id === owner.id)?.gross_total).toBe(hole.par)
      measurements.push({ state: round.scoring_format, metric: 'gross', elapsedMs: Date.now() - started, requests: [...requests] })
      if (round.id === first.id) {
        expectedSaveFailure = true
        const path = `**/api/rounds/${round.id}/scores`
        await page.route(path, route => route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Test: lagring utilgjengelig' } } }))
        await page.getByRole('button', { name: 'Legg til ett slag' }).click()
        await expect(page.locator('.score-save-error')).toBeVisible()
        expect((await scorecard(api, round, owner)).holes[0]?.score?.gross_strokes).toBe(hole.par)
        expect(await tournamentBoard(phone.request, trip.id, 'gross')).toEqual(board)
        await assertRendered(phone, board)
        await layout(page, 'failed-save')
        await page.unroute(path)
        await page.getByRole('button', { name: 'Forkast' }).click()
        expectedSaveFailure = false
      }
      await phone.getByRole('button', { name: 'Netto', exact: true }).click()
      await assertRendered(phone, await tournamentBoard(phone.request, trip.id, 'net'))
      requests.length = 0
      const next = nextBoard(phone, trip.id, 'net')
      await page.getByRole('button', { name: 'Legg til ett slag' }).click()
      await expect(page.locator('.score-sync')).toHaveText(/^(Synkronisert|Lagret)$/)
      await next
      await assertRendered(phone, await tournamentBoard(phone.request, trip.id, 'net'))
      expect(requests).toEqual([`/api/tournaments/${trip.id}/rounds`, `/api/tournaments/${trip.id}/leaderboards/net`])
      measurements.push({ state: round.scoring_format, metric: 'net', requests: [...requests] })
      // Returning within the scoring session uses the same canonical gross/net results.
      await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Resultater' }).click()
      await page.getByRole('button', { name: 'Turnering', exact: true }).click()
      await page.getByRole('button', { name: 'Netto', exact: true }).click()
      await assertRendered(page, await tournamentBoard(api, trip.id, 'net'))
      await layout(phone, round.scoring_format)
    }
    // A lower open round changes the round result, but not the tournament selection.
    const owner = (await progress(api, first)).owners[0]?.owner
    if (!owner) throw new Error('Missing first owner')
    const card = await scorecard(api, first, owner); const hole = card.holes[0]
    if (!hole) throw new Error('Missing first hole')
    const before = await tournamentBoard(phone.request, trip.id, 'net')
    const refresh = nextBoard(phone, trip.id, 'net')
    await mutate(api, `/api/rounds/${first.id}/scores`, auth.csrf_token, { owner, hole_id: hole.hole_id, gross_strokes: 9 }, 'PUT')
    await refresh
    expect(await tournamentBoard(phone.request, trip.id, 'net')).toEqual(before)
    expect((await roundBoard(api, first, 'gross')).entries.find(entry => entry.owner.id === owner.id)?.gross_total).toBe(9)
    await assertRendered(phone, before)
    measurements.push({ state: 'older-open-excluded', unchanged: true })
    // Complete a historical team round; correction changes preserved player contributions.
    await fillAndConfirm(api, first, auth.csrf_token, 4)
    await mutate(api, `/api/rounds/${first.id}/complete`, auth.csrf_token)
    await assertRendered(phone, await tournamentBoard(phone.request, trip.id, 'net'))
    const historical = await tournamentBoard(phone.request, trip.id, 'net')
    const correction = nextBoard(phone, trip.id, 'net')
    await mutate(api, `/api/rounds/${first.id}/scores`, auth.csrf_token, { owner, hole_id: hole.hole_id, gross_strokes: 5 }, 'PUT')
    await correction
    const corrected = await tournamentBoard(phone.request, trip.id, 'net')
    expect(corrected).not.toEqual(historical)
    await assertRendered(phone, corrected)
    await mutate(api, `/api/rounds/${first.id}/scorecards/${owner.type}/${owner.id}/confirm`, auth.csrf_token)
    await mutate(api, `/api/rounds/${first.id}/lock`, auth.csrf_token)
    const locked = await api.put(`/api/rounds/${first.id}/scores`, { headers: { 'x-csrf-token': auth.csrf_token }, data: { owner, hole_id: hole.hole_id, gross_strokes: 7 } })
    expect(locked.status()).toBe(409)
    expect(await tournamentBoard(phone.request, trip.id, 'net')).toEqual(corrected)
    // More history than optional slots: deliberately poor completed round is uncounted.
    await fillAndConfirm(api, individual, auth.csrf_token, 10)
    await mutate(api, `/api/rounds/${individual.id}/complete`, auth.csrf_token)
    await assertRendered(phone, await tournamentBoard(phone.request, trip.id, 'net'))
    const individualOwner = (await progress(api, individual)).owners[0]?.owner
    if (!individualOwner) throw new Error('Missing individual owner')
    const individualCard = await scorecard(api, individual, individualOwner)
    const individualHole = individualCard.holes[0]
    if (!individualHole) throw new Error('Missing individual hole')
    const excludedBefore = await tournamentBoard(phone.request, trip.id, 'net')
    expect(excludedBefore.entries.find(entry => entry.player_id === individualOwner.id)?.contributions.find(item => item.round_id === individual.id)?.counted).toBe(false)
    const uncountedUpdate = nextBoard(phone, trip.id, 'net')
    await mutate(api, `/api/rounds/${individual.id}/scores`, auth.csrf_token, { owner: individualOwner, hole_id: individualHole.hole_id, gross_strokes: 11 }, 'PUT')
    await uncountedUpdate
    const excludedAfter = await tournamentBoard(phone.request, trip.id, 'net')
    expect(excludedAfter.entries.map(entry => entry.net_total)).toEqual(excludedBefore.entries.map(entry => entry.net_total))
    expect(excludedAfter).not.toEqual(excludedBefore)
    await assertRendered(phone, excludedAfter)
    measurements.push({ state: 'best-N-excluded', totalUnchanged: true, contributionUpdated: true })
    // Disconnect must hide projections; reconnect must retrieve changes committed offline.
    expectedOutage = true
    await phoneContext.setOffline(true)
    await server.stop()
    await expect(phone.locator('.leaderboard-row-link')).toHaveCount(0, { timeout: 20000 })
    await layout(phone, 'disconnected')
    await server.start()
    const writable = (await progress(api, foursomes)).owners[0]?.owner
    if (!writable) throw new Error('Missing foursomes owner')
    const writableCard = await scorecard(api, foursomes, writable); const targetHole = writableCard.holes[0]
    if (!targetHole) throw new Error('Missing foursomes hole')
    await mutate(api, `/api/rounds/${foursomes.id}/scores`, auth.csrf_token, { owner: writable, hole_id: targetHole.hole_id, gross_strokes: 8 }, 'PUT')
    await phoneContext.setOffline(false)
    await assertRendered(phone, await tournamentBoard(phone.request, trip.id, 'net'))
    expectedOutage = false
    // Member session verifies hidden final scores never enter tournament projections.
    await phone.getByRole('button', { name: 'Logg ut' }).click()
    await login(phone, 'anders')
    await mutate(api, `/api/rounds/${final.id}/open`, auth.csrf_token)
    const finalOwner = (await progress(api, final)).owners[0]?.owner
    if (!finalOwner) throw new Error('Missing final owner')
    const finalCard = await scorecard(api, final, finalOwner)
    const front = finalCard.holes[0]; const back = finalCard.holes[9]
    if (!front || !back) throw new Error('Missing final holes')
    await phone.goto(`/leaderboard?tournament=${trip.id}&scope=tournament&metric=net`)
    await assertRendered(phone, await tournamentBoard(phone.request, trip.id, 'net'))
    for (const selected of [front, back]) {
      const event = nextBoard(phone, trip.id, 'net')
      await mutate(api, `/api/rounds/${final.id}/scores`, auth.csrf_token, { owner: finalOwner, hole_id: selected.hole_id, gross_strokes: 6 }, 'PUT')
      await event
      await assertRendered(phone, await tournamentBoard(phone.request, trip.id, 'net'))
    }
    const hidden = await tournamentBoard(phone.request, trip.id, 'net')
    expect(hidden.visibility.mode).toBe('front_nine')
    expect(hidden.entries.find(entry => entry.player_id === finalOwner.id)?.contributions.find(item => item.round_id === final.id)).toMatchObject({ holes_scored: 1, gross_total: 6, mandatory: true, counted: true })
    for (const hide of [false, true]) {
      const path = `/api/tournaments/${trip.id}/final-round-visibility`
      const setting = decodeObject(await (await api.get(path)).json(), 'visibility')
      const event = nextBoard(phone, trip.id, 'net')
      await mutate(api, path, auth.csrf_token, { back_nine_hidden: hide, expected_visibility_updated_at: setting.visibility_updated_at }, 'PATCH')
      await event
      const board = await tournamentBoard(phone.request, trip.id, 'net')
      expect(board.visibility.mode).toBe(hide ? 'front_nine' : 'full')
      expect(board.entries.find(entry => entry.player_id === finalOwner.id)?.contributions.find(item => item.round_id === final.id)?.holes_scored).toBe(hide ? 1 : 2)
      await assertRendered(phone, board)
      await layout(phone, hide ? 're-hidden' : 'released')
    }
    expect(failures).toEqual([])
  } finally {
    await writeFile(process.env.GOLF_LEADERBOARD_MEASUREMENTS ?? '/tmp/golf-leaderboard-measurements.json', JSON.stringify(measurements, null, 2))
    await phoneContext.close()
  }
})
