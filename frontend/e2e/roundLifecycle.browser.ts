import { test, expect, type APIRequestContext, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeRound, decodeTournamentList, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeCompletionValidation, decodeScoringScorecard, ownerTypeForFormat } from '../src/api/scorecards'
import type { Round } from '../src/api/types'

// Requires a freshly migrated/seeded disposable database behind the local API.
// The explicit opt-in prevents ordinary frontend test runs from mutating a database.
test.skip(process.env.GOLF_LIFECYCLE_BROWSER !== '1', 'Set GOLF_LIFECYCLE_BROWSER=1 only with a disposable seeded local database.')

async function read<T>(api: APIRequestContext, path: string, decode: (value: unknown) => T): Promise<T> {
  const response = await api.get(path)
  expect(response.ok(), `${path}: ${response.status()}`).toBe(true)
  const value: unknown = await response.json()
  return decode(value)
}

async function mutation(api: APIRequestContext, path: string, csrf: string, data: unknown = {}, method = 'POST') {
  const response = await api.fetch(path, { method, data, headers: { 'x-csrf-token': csrf } })
  expect(response.ok(), `${path}: ${response.status()}`).toBe(true)
}

async function login(page: Page, username: string) {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill(username)
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
}

async function layout(page: Page, width: number) {
  await page.setViewportSize({ width, height: 900 })
  const dimensions = await page.evaluate(() => ({ width: window.innerWidth, content: document.documentElement.scrollWidth }))
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.width)
  for (const button of await page.locator('.round-lifecycle button:visible').all()) {
    const box = await button.boundingBox()
    expect(box?.height).toBeGreaterThanOrEqual(44)
  }
}

async function transition(page: Page, label: string) {
  const action = page.getByRole('button', { name: label, exact: true })
  await expect(action).toBeEnabled()
  await action.click()
  await expect(page.getByRole('button', { name: 'Avbryt', exact: true })).toBeFocused()
  await page.getByRole('button', { name: `Bekreft og ${label.toLocaleLowerCase('nb-NO')}`, exact: true }).click()
}

async function fillCards(api: APIRequestContext, round: Round, csrf: string) {
  const progress = await read(api, `/api/rounds/${round.id}/completion-validation`,
    (value) => decodeCompletionValidation(value, round.id, ownerTypeForFormat(round.scoring_format)))
  for (const [index, item] of progress.owners.entries()) {
    const path = `/api/rounds/${round.id}/scorecards/${item.owner.type}/${item.owner.id}`
    const card = await read(api, `${path}/scoring`, (value) => decodeScoringScorecard(value, round.id, item.owner))
    for (const hole of card.holes) {
      if (!hole.score) await mutation(api, `/api/rounds/${round.id}/scores`, csrf,
        { hole_id: hole.hole_id, owner: item.owner, gross_strokes: hole.par }, 'PUT')
    }
    // Leave the first card for the real confirmation UI.
    if (index > 0) await mutation(api, `${path}/confirm`, csrf)
  }
  return progress.owners[0]
}

test('admin lifecycle, live readiness, corrections, permissions and final visibility', async ({ page, browser }) => {
  const errors: string[] = []
  const failed: string[] = []
  const errorResponses: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('requestfailed', (request) => {
    if (!request.url().includes('/live') && !request.failure()?.errorText.includes('ERR_ABORTED')) failed.push(new URL(request.url()).pathname)
  })
  page.on('response', (response) => { if (response.status() >= 400) errorResponses.push(`${response.status()} ${new URL(response.url()).pathname}`) })
  await login(page, 'admin')
  expect(errorResponses.filter((entry) => entry !== '401 /api/auth/session')).toEqual([])
  errorResponses.length = 0
  page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
  const api = page.context().request
  const auth = await read(api, '/api/auth/session', decodeAuthSession)
  const trips = await read(api, '/api/tournaments', decodeTournamentList)
  const trip = trips.find((item) => item.name === 'Guttas Golf 2026')
  expect(trip, 'Fresh seed tournament required').toBeDefined()
  if (!trip) return
  const rounds = await read(api, `/api/tournaments/${trip.id}/rounds`, (value) => decodeTournamentRounds(value, trip.id))
  expect(['draft', 'active']).toContain(trip.status)
  expect(rounds.every((round) => round.status === 'draft')).toBe(true)
  const first = rounds[0]
  if (!first) throw new Error('Seed round missing')
  const manage = (id: string) => `/manage/tournaments/${trip.id}?round=${id}#lifecycle`
  await page.goto(manage(first.id))
  if (trip.status === 'draft') {
    await expect(page.getByRole('button', { name: 'Åpne runden', exact: true })).toBeDisabled()
    await expect(page.getByText('Start turneringen før runden åpnes.')).toBeVisible()
    await layout(page, 320)
    await page.getByRole('button', { name: 'Start turneringen', exact: true }).click()
  }
  await expect(page.getByRole('heading', { name: 'Turneringen er startet' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Åpne runden', exact: true })).toBeEnabled()
  await page.locator('.round-lifecycle').screenshot({ path: '/tmp/golf-lifecycle-ready-mobile.png' })

  // Start with an unsaved manual draft and prove opening preserves it disabled.
  await page.getByRole('link', { name: 'Baner', exact: true }).click()
  const course = page.locator('.round-course-card').first()
  const edit = course.getByRole('button', { name: 'Endre', exact: true })
  if (await edit.getAttribute('aria-expanded') !== 'true') await edit.click()
  await course.getByLabel('Registrer manuelt', { exact: true }).check()
  const courseName = course.getByLabel('Banenavn', { exact: true })
  await courseName.fill('Ulagret baneutkast')
  await page.getByRole('link', { name: 'Livsløp', exact: true }).click()
  await page.getByRole('button', { name: 'Åpne runden', exact: true }).click()
  await page.keyboard.press('Escape')
  await expect(page.getByRole('button', { name: 'Åpne runden', exact: true })).toBeFocused()
  await transition(page, 'Åpne runden')
  await expect(page.getByText('Runden er åpnet.', { exact: true })).toBeVisible()
  await expect(courseName).toHaveValue('Ulagret baneutkast')
  await expect(courseName).toBeDisabled()
  await layout(page, 1440)

  // Exercise all supported owner formats and the 18-hole final.
  for (const index of [0, 1, 2, 4]) {
    console.info(`Validating lifecycle for seed round ${index + 1}`)
    const round = rounds[index]
    if (!round) throw new Error('Seed round missing')
    await page.goto(manage(round.id))
    if (index !== 0) await transition(page, 'Åpne runden')
    await expect(page.getByRole('button', { name: 'Fullfør runden', exact: true })).toBeDisabled()
    if (index === 0) {
      await page.getByRole('link', { name: /^Fyll ut scorekort for / }).first().click()
      await page.locator('.scorecard-holes li').first().getByRole('button').click()
      await page.getByRole('button', { name: /^Registrer par/ }).click()
      await expect(page.locator('.score-sync')).toHaveText(/^(Lagret|Synkronisert)$/)
      await page.goto(manage(round.id))
    }
    const firstOwner = await fillCards(api, round, auth.csrf_token)
    if (!firstOwner) throw new Error('Required owner missing')
    // The mounted UI must update from real score SSE events without a reload.
    const confirmLink = page.getByRole('link', { name: `Bekreft scorekort for ${firstOwner.owner_name}`, exact: true })
    await expect(confirmLink).toBeVisible()
    await confirmLink.click()
    await expect(page).toHaveURL(new RegExp(`owner=${firstOwner.owner.id}`))
    await page.getByRole('button', { name: 'Bekreft fullført scorekort', exact: true }).click()
    await expect(page.getByText('Scorekortet er bekreftet', { exact: true })).toBeVisible()
    await page.goto(manage(round.id))
    await layout(page, index === 1 ? 390 : 1440)
    await transition(page, 'Fullfør runden')
    await expect(page.getByText('Runden er fullført.', { exact: true })).toBeVisible()
    if (index === 0) {
      // A real correction from another admin session invalidates confirmation.
      const other = await browser.newContext({ baseURL: 'http://127.0.0.1:5173' })
      const otherPage = await other.newPage()
      await login(otherPage, 'admin')
      await otherPage.goto(`/score?${new URLSearchParams({ tournament: trip.id, round: round.id, owner_type: firstOwner.owner.type, owner: firstOwner.owner.id, view: 'summary' })}`)
      await otherPage.getByRole('button', { name: 'Korriger score', exact: true }).click()
      await otherPage.locator('.scorecard-holes li').first().getByRole('button').click()
      await otherPage.getByRole('button', { name: 'Legg til ett slag', exact: true }).click()
      await expect(otherPage.locator('.score-sync')).toHaveText(/^(Lagret|Synkronisert)$/)
      await expect(page.getByRole('button', { name: 'Lås runden', exact: true })).toBeDisabled()
      await page.getByRole('link', { name: `Bekreft scorekort for ${firstOwner.owner_name}`, exact: true }).click()
      await page.getByRole('button', { name: 'Bekreft fullført scorekort', exact: true }).click()
      await expect(page.getByText('Scorekortet er bekreftet', { exact: true })).toBeVisible()
      await page.goto(manage(round.id))
      // Another admin locks while this page remains mounted; live state must follow.
      await otherPage.goto(manage(round.id))
      await transition(otherPage, 'Lås runden')
      await expect(page.getByText('Runden er låst. Scorekortene er skrivebeskyttet.')).toBeVisible()
      await other.close()
    } else {
      await transition(page, 'Lås runden')
      await expect(page.getByText('Runden er låst.', { exact: true })).toBeVisible()
    }
    expect((await read(api, `/api/rounds/${round.id}`, (value) => decodeRound(value))).status).toBe('locked')
    await layout(page, 320)
  }
  await page.locator('.round-lifecycle').screenshot({ path: '/tmp/golf-lifecycle-locked-mobile.png' })
  const final = rounds[4]
  if (!final) throw new Error('Final missing')
  const visibilityResponse = await api.get(`/api/tournaments/${trip.id}/final-round-visibility`)
  const visibility: unknown = await visibilityResponse.json()
  expect(visibility).toMatchObject({ back_nine_hidden: true })
  const member = await browser.newContext({ baseURL: 'http://127.0.0.1:5173' })
  const memberPage = await member.newPage()
  await login(memberPage, 'anders')
  await memberPage.goto(manage(final.id))
  await expect(memberPage.getByRole('heading', { name: 'Ingen tilgang' })).toBeVisible()
  expect(await memberPage.locator('.round-lifecycle').count()).toBe(0)
  const redacted = await read(member.request, `/api/rounds/${final.id}/completion-validation`,
    (value) => decodeCompletionValidation(value, final.id, ownerTypeForFormat(final.scoring_format)))
  expect(redacted.visibility.mode).toBe('front_nine')
  expect(redacted.ready_to_lock).toBeNull()
  await memberPage.goto(`/rounds/${final.id}`)
  expect(await memberPage.getByRole('link', { name: 'Administrer runden' }).count()).toBe(0)
  await member.close()
  expect(errors).toEqual([])
  expect(failed).toEqual([])
  expect(errorResponses).toEqual([])
})

test('loading, retry, empty and long-content states at mobile and desktop widths', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await login(page, 'admin')
  const api = page.context().request
  const trips = await read(api, '/api/tournaments', decodeTournamentList)
  const trip = trips.find((item) => item.name === 'Guttas Golf 2026')
  if (!trip) throw new Error('Seed tournament missing')
  const rounds = await read(api, `/api/tournaments/${trip.id}/rounds`, (value) => decodeTournamentRounds(value, trip.id))
  const round = rounds[3]
  if (!round || round.status !== 'draft') throw new Error('Seed round four must remain draft')
  const url = `/manage/tournaments/${trip.id}?round=${round.id}#lifecycle`
  let release: (() => void) | undefined
  const pending = new Promise<void>((resolve) => { release = resolve })
  let fail = false
  await page.route(`**/api/rounds/${round.id}/pairing-validation`, async (route) => {
    await pending
    if (fail) await route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: { code: 'unavailable', message: 'Test: midlertidig feil' } }) })
    else await route.continue()
  })
  await page.goto(url)
  const panel = page.locator('.round-lifecycle')
  await expect(panel.getByText('Laster …', { exact: true })).toBeVisible()
  await expect(panel.getByRole('button', { name: 'Åpne runden', exact: true })).toBeDisabled()
  await layout(page, 320)
  release?.()
  await expect(panel.getByRole('button', { name: 'Åpne runden', exact: true })).toBeEnabled()
  fail = true
  await panel.getByRole('button', { name: 'Oppdater kontrollen', exact: true }).click()
  await expect(panel.getByText(/Kunne ikke kontrollere rundestatus/)).toBeVisible()
  await expect(panel.getByRole('button', { name: 'Åpne runden', exact: true })).toBeDisabled()
  await layout(page, 1440)
  fail = false
  await panel.getByRole('button', { name: 'Oppdater kontrollen', exact: true }).click()
  await expect(panel.getByRole('button', { name: 'Åpne runden', exact: true })).toBeEnabled()
  await page.unroute(`**/api/rounds/${round.id}/pairing-validation`)

  const longRound = { ...round, name: 'En veldig lang runde med et utførlig navn '.repeat(8), status: 'open' }
  await page.route(`**/api/rounds/${round.id}`, (route) => route.fulfill({ json: longRound }))
  await page.route(`**/api/tournaments/${trip.id}/rounds`, (route) => route.fulfill({ json: rounds.map((item) => item.id === round.id ? longRound : item) }))
  await page.route(`**/api/rounds/${round.id}/completion-validation`, (route) => route.fulfill({ json: {
    round_id: round.id, status: 'open', visibility: { mode: 'full' }, owners: [],
    ready_to_complete: false, ready_to_lock: false,
    issues: [{ code: 'no_required_owners', message: 'empty' }, { code: 'round_not_completed', message: 'open' }],
  } }))
  await page.reload()
  await expect(panel.getByText('Runden har ingen nødvendige scorekort og kan ikke fullføres eller låses.')).toBeVisible()
  await expect(panel.getByRole('button', { name: 'Fullfør runden', exact: true })).toBeDisabled()
  for (const width of [320, 390, 1440]) await layout(page, width)
  await panel.screenshot({ path: '/tmp/golf-lifecycle-long-empty-desktop.png' })
  await page.unroute(`**/api/tournaments/${trip.id}/rounds`)
  await page.route(`**/api/tournaments/${trip.id}/rounds`, (route) => route.fulfill({ json: [] }))
  await page.reload()
  await expect(page.getByText('Ingen runder er opprettet.').first()).toBeVisible()
  expect(await panel.count()).toBe(0)
  await layout(page, 320)
  expect(errors).toEqual([])
})
