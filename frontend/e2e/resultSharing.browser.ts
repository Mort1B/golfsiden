import { test, expect } from '@playwright/test'
import { decodeObject } from '../src/api/decoder'
import { decodeAuthSession } from '../src/api/auth'
import { decodeRound, decodeTournament, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodePublicResults } from '../src/api/resultSharing/decoders'

import { decodeScoringScorecard } from '../src/api/scorecards'
import { capability, sharingEvents, sharingLayout } from './resultSharingSupport'

test.skip(process.env.GOLF_RESULT_SHARING_BROWSER !== '1', 'Requires disposable local API and explicit GOLF_RESULT_SHARING_BROWSER=1.')
test.use({ screenshot: 'off', trace: 'off' })
test('real admin issue/copy/replace/revoke and anonymous live projection ignore admin cookies and hidden final scores', async ({ page, browser }) => {
  const stamp = Date.now(); const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
  const response = await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username: `sharing_${stamp}`, password: 'sharing-browser-password' }, player: { display_name: 'Arrangør med et svært langt spillernavn i en åpen golfturnering', handicap_index: 0 } },
    tournament: { name: `Offentlig golfturnering ${stamp}`, description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
    rounds: [{ round_number: 1, name: 'Finalen', round_date: day, scoring_format: 'individual_stroke_play' }],
  } })
  expect(response.status()).toBe(201)
  const body = decodeObject(await response.json(), 'onboarding'); const trip = decodeTournament(body.tournament); const auth = decodeAuthSession(body.session)
  const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${trip.id}/rounds`)).json(), trip.id)
  const first = rounds[0]; if (!first || !auth.player_id) throw new Error('Missing created round/player')
  async function mutate(path: string, data: unknown = {}, method = 'POST') {
    const result = await page.request.fetch(path, { method, data, headers: { 'x-csrf-token': auth.csrf_token } })
    expect(result.ok(), `Mutation status ${result.status()}`).toBe(true); return result
  }
  const round = decodeRound(await (await mutate(`/api/rounds/${first.id}/course-configuration`, {
    expected_round_updated_at: first.updated_at, selection: { source: 'manual', course_name: 'Testbane', location: null,
      tee: { category: 'male', name: 'Gul', course_rating: 72, slope_rating: 113, holes: Array.from({ length: 18 }, (_, index) => ({ par: 4, stroke_index: index + 1, distance: null })) } },
  }, 'PUT')).json())
  await mutate(`/api/rounds/${round.id}/pairings`, { expected_round_updated_at: round.updated_at, teams: [], flights: [{ id: crypto.randomUUID(), name: 'Flight 1', starting_hole: 1, tee_time: null, members: [{ player_id: auth.player_id }] }], legacy_conversions: [] }, 'PUT')
  const fresh = decodeTournament(await (await page.request.get(`/api/tournaments/${trip.id}`)).json())
  await mutate(`/api/tournaments/${trip.id}/start`, { expected_tournament_updated_at: fresh.updated_at })
  await mutate(`/api/rounds/${round.id}/open`)
  const owner = { type: 'player' as const, id: auth.player_id }
  const card = decodeScoringScorecard(await (await page.request.get(`/api/rounds/${round.id}/scorecards/player/${owner.id}/scoring`)).json(), round.id, owner)
  const adminEvents = sharingEvents(page)
  const sharePath = `/api/tournaments/${trip.id}/result-share`
  let metadataMode: 'loading' | 'error' | 'ready' = 'loading'
  let releaseMetadata: (() => void) | undefined
  const metadataWait = new Promise<void>(resolve => { releaseMetadata = resolve })
  await page.route(`**${sharePath}`, async route => {
    if (route.request().method() !== 'GET') return route.continue()
    if (metadataMode === 'loading') await metadataWait
    if (metadataMode === 'error') return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'unavailable' } } })
    await route.continue()
  })
  await page.goto(`/manage/tournaments/${trip.id}#settings`)
  await expect(page.getByText('Henter status for resultatdeling …')).toBeVisible()
  await sharingLayout(page, 'admin-loading', '.result-share-control')
  metadataMode = 'error'; releaseMetadata?.()
  await expect(page.getByText('Kunne ikke hente status for resultatdeling.')).toBeVisible()
  await sharingLayout(page, 'admin-error', '.result-share-control')
  metadataMode = 'ready'; await page.getByRole('button', { name: 'Oppdater delingsstatus' }).click()
  await expect(page.getByText('Ingen resultatlenke er opprettet.')).toBeVisible()
  await page.unroute(`**${sharePath}`)
  await sharingLayout(page, 'admin-empty', '.result-share-control')
  await page.getByRole('button', { name: 'Opprett offentlig resultatlenke' }).click()
  const link = page.getByLabel('Offentlig resultatlenke', { exact: true })
  await expect(link).toBeVisible(); const original = await link.inputValue(); const grant = capability(original)
  await page.context().grantPermissions(['clipboard-read', 'clipboard-write'])
  await page.getByRole('button', { name: 'Kopier resultatlenke' }).click()
  expect(await page.evaluate(async value => await navigator.clipboard.readText() === value, original)).toBe(true)
  await page.evaluate(() => { Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async () => { throw new Error('unavailable') } } }) })
  await page.getByRole('button', { name: 'Kopier resultatlenke' }).click()
  await expect(page.getByText(/Kunne ikke kopiere/)).toBeVisible()
  await sharingLayout(page, 'admin-receipt', '.result-share-control')
  const context = await browser.newContext(); const publicPage = await context.newPage(); const publicEvents = sharingEvents(publicPage)
  const adminPublic = await page.context().newPage(); const cookieEvents = sharingEvents(adminPublic)
  try {
    await publicPage.goto(original); await expect(publicPage.getByRole('heading', { name: trip.name, exact: true })).toBeVisible()
    expect((await context.cookies()).some(cookie => cookie.name.includes('session'))).toBe(false)
    await expect(publicPage.locator('a')).toHaveCount(0)
    const hole = card.holes[0]; if (!hole) throw new Error('Missing hole')
    await mutate(`/api/rounds/${round.id}/scores`, { owner, hole_id: hole.hole_id, gross_strokes: 5 }, 'PUT')
    await expect(publicPage.locator('.public-result-score strong')).toHaveText('+1', { timeout: 20_000 })
    await sharingLayout(publicPage, 'anonymous-live', '.public-results-page')
    for (const item of card.holes.slice(1)) await mutate(`/api/rounds/${round.id}/scores`, { owner, hole_id: item.hole_id, gross_strokes: 4 }, 'PUT')
    async function visibility(hidden: boolean) {
      const current = decodeObject(await (await page.request.get(`/api/tournaments/${trip.id}/final-round-visibility`)).json(), 'visibility')
      await mutate(`/api/tournaments/${trip.id}/final-round-visibility`, { back_nine_hidden: hidden, expected_visibility_updated_at: current.visibility_updated_at }, 'PATCH')
    }
    await visibility(false)
    await publicPage.getByRole('button', { name: 'Oppdater resultater' }).click()
    await expect(publicPage.locator('.public-result-score span')).toHaveText('73 brutto')
    await visibility(true)
    await publicPage.evaluate(() => document.dispatchEvent(new Event('visibilitychange')))
    await expect(publicPage.locator('.public-result-score span')).toHaveText('37 brutto')
    await adminPublic.goto(original)
    await expect(adminPublic.locator('.public-result-score span')).toHaveText('37 brutto')
    await sharingLayout(adminPublic, 'admin-cookie-hidden', '.public-results-page')
    const rawRead = async () => decodePublicResults(await (await page.request.post(`/api/public/results/${grant.id}`, { data: { token: grant.token, metric: 'gross' } })).json(), grant.id, 'gross')
    const before = await rawRead(); const last = card.holes.at(-1); if (!last) throw new Error('Missing last hole')
    await mutate(`/api/rounds/${round.id}/scores`, { owner, hole_id: last.hole_id, gross_strokes: 9 }, 'PUT')
    expect(await rawRead()).toEqual(before)
    await mutate(`/api/rounds/${round.id}/scorecards/player/${owner.id}/confirm`)
    await mutate(`/api/rounds/${round.id}/complete`)
    await publicPage.getByRole('button', { name: 'Oppdater resultater' }).click()
    await expect(publicPage.locator('.public-result-score strong')).toHaveText('–')
    await sharingLayout(publicPage, 'hidden-completed-final', '.public-results-page')
    // A competing admin rotates first; this browser must drop its old receipt
    // and refresh metadata rather than unknowingly overwriting the new grant.
    await page.route(`**${sharePath}`, async route => {
      if (route.request().method() !== 'POST') return route.continue()
      const status = decodeObject(await (await page.request.get(sharePath)).json(), 'status')
      const latest = decodeObject(status.grant, 'grant')
      const concurrent = await page.request.post(sharePath, { headers: { 'x-csrf-token': auth.csrf_token }, data: { expected_grant_id: latest.id } })
      expect(concurrent.status()).toBe(201)
      await route.continue()
    })
    await page.getByRole('button', { name: 'Erstatt resultatlenken' }).click()
    await expect(page.getByText(/Lenken ble endret et annet sted/)).toBeVisible()
    await expect(link).toHaveCount(0)
    await sharingLayout(page, 'admin-stale', '.result-share-control')
    await page.unroute(`**${sharePath}`)
    let releaseIssue: (() => void) | undefined
    const issueWait = new Promise<void>(resolve => { releaseIssue = resolve })
    await page.route(`**${sharePath}`, async route => { if (route.request().method() === 'POST') await issueWait; await route.continue() })
    await page.getByRole('button', { name: 'Erstatt resultatlenken' }).click()
    await expect(page.getByText('Oppdaterer resultatdeling …')).toBeVisible()
    await sharingLayout(page, 'admin-pending', '.result-share-control')
    releaseIssue?.()
    await expect(link).toBeVisible(); const replacement = await link.inputValue()
    expect(replacement === original).toBe(false)
    await publicPage.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
    await expect(publicPage.getByRole('alert')).toContainText('ikke tilgjengelig')
    await sharingLayout(publicPage, 'replaced', '.public-results-page')
    await publicPage.goto(replacement); await expect(publicPage.getByRole('heading', { name: trip.name, exact: true })).toBeVisible()
    await page.getByRole('button', { name: 'Tilbakekall resultatlenken' }).click()
    await expect(page.getByText('Resultatlenken er tilbakekalt.', { exact: true })).toBeVisible()
    await sharingLayout(page, 'admin-revoked', '.result-share-control')
    await publicPage.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
    await expect(publicPage.getByRole('alert')).toContainText('ikke tilgjengelig')
    expect(publicEvents.privateReads).toEqual([]); expect(cookieEvents.privateReads).toEqual([])
    for (const events of [adminEvents, publicEvents, cookieEvents]) {
      expect(events.errors).toEqual([]); expect(events.statuses.every(status => status === 401 || status === 404 || status === 409 || status === 503)).toBe(true)
      expect(events.failed.every(error => error === 'net::ERR_ABORTED')).toBe(true)
    }
  } finally { await adminPublic.close(); await context.close() }
})
