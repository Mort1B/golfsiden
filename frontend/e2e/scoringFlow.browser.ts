import { test, expect, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeTournamentList, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeCompletionValidation, decodeReadScorecard, ownerTypeForFormat, type ScoringScorecard } from '../src/api/scorecards'
import { LIVE_RESULTS_EXPLANATION, MANDATORY_ROUND_EXPLANATION } from '../src/features/leaderboards/resultExplanations'

test.skip(process.env.GOLF_SCORING_FLOW_BROWSER !== '1', 'Requires disposable seeded local services and GOLF_SCORING_FLOW_BROWSER=1.')
async function layout(page: Page, state: string) {
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    await page.evaluate(() => window.scrollTo(0, 0))
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const control of await page.locator('.score-page button:visible, .score-page select:visible, .score-page summary:visible').all()) {
      expect((await control.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await control.isEnabled()) await control.click({ trial: true })
    }
    await page.evaluate(() => window.scrollTo(0, 0))
    await page.screenshot({ path: `/tmp/golf-scoring-${state}-${width}.png`, fullPage: true })
  }
}

test('resume fresh gaps, explicit history, quick cards, guarded saves, async states and responsive composition', async ({ page }) => {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill('admin')
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
  const auth = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
  const trips = decodeTournamentList(await (await page.request.get('/api/tournaments')).json())
  const trip = trips.find(item => item.name === 'Guttas Golf 2026')
  if (!trip) throw new Error('Missing seed tournament')
  const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${trip.id}/rounds`)).json(), trip.id)
  const round = rounds[0]
  if (!round) throw new Error('Missing seed round')
  const progress = decodeCompletionValidation(await (await page.request.get(`/api/rounds/${round.id}/completion-validation`)).json(), round.id, ownerTypeForFormat(round.scoring_format))
  const first = progress.owners[0]; const second = progress.owners[1]
  if (!first || !second) throw new Error('Missing seed owners')
  const source = decodeReadScorecard(await (await page.request.get(`/api/rounds/${round.id}/scorecards/${first.owner.type}/${first.owner.id}`)).json(), round.id, first.owner)
  const errors: string[] = []; const failed: string[] = []
  page.on('pageerror', error => errors.push(error.message))
  page.on('console', msg => { if (msg.type() === 'error' && !msg.text().startsWith('Failed to load resource:')) errors.push(msg.text()) })
  page.on('requestfailed', req => { if (!req.url().includes('/live') && !req.failure()?.errorText.includes('ERR_ABORTED')) failed.push(req.url()) })
  page.on('response', response => { if (response.status() >= 400 && response.status() !== 503) failed.push(`${response.status()} ${new URL(response.url()).pathname}`) })
  let scored = [1, 3]; let fail = false; let empty = false
  let pending: Promise<void> | null = null
  const longName = 'En turnering med et langt navn for å kontrollere at mobilvisningen fortsatt er brukbar'
  await page.route('**/api/tournaments', route => route.fulfill({ json: [{ ...trip, name: longName }] }))
  await page.route(`**/api/tournaments/${trip.id}/rounds`, route => route.fulfill({ json: rounds.map(item => ({ ...item, status: 'open' })) }))
  await page.route(`**/api/rounds/${round.id}/completion-validation`, route => route.fulfill({ json: { ...progress, status: 'open', ready_to_lock: false, ready_to_complete: !empty,
    owners: empty ? [] : progress.owners.map(item => ({ ...item, owner_name: `${item.owner_name} med et veldig langt navn for mobilvisningen`, holes_scored: 18, complete: true, confirmed: true })),
    issues: [...(empty ? [{ code: 'no_required_owners', message: 'none' }] : []), { code: 'round_not_completed', message: 'open' }] } }))
  await page.route(`**/api/rounds/${round.id}/score-access`, route => route.fulfill({ json: { round_id: round.id, writable_owners: progress.owners.map(item => item.owner) } }))
  await page.route(`**/api/rounds/${round.id}/scorecards/*/*/scoring`, async route => {
    if (pending) await pending
    if (fail) { await route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Midlertidig utilgjengelig' } } }); return }
    const requestedOwner = progress.owners.find(item => route.request().url().includes(item.owner.id))?.owner
    if (!requestedOwner) throw new Error('Unexpected card owner')
    const holes = source.holes.map(hole => ({ ...hole, net_strokes: scored.includes(hole.hole_number) ? hole.par : null,
      score: scored.includes(hole.hole_number) ? { id: hole.hole_id, round_id: round.id, hole_id: hole.hole_id, owner: requestedOwner,
        gross_strokes: hole.par, submitted_by: auth.user_id, submitted_at: trip.created_at, updated_at: trip.updated_at } : null }))
    const total = holes.reduce((sum, hole) => sum + (hole.score?.gross_strokes ?? 0), 0)
    const card: ScoringScorecard = { projection: 'scoring', round_id: round.id, owner: requestedOwner, holes, gross_total: total, net_total: total,
      playing_handicap: 0, holes_scored: scored.length, number_of_holes: source.number_of_holes, complete: scored.length === source.number_of_holes, confirmed: false, confirmed_at: null, confirmed_by: null }
    await route.fulfill({ json: card })
  })
  const explicit = `/score?tournament=${trip.id}&round=${round.id}&owner_type=${first.owner.type}&owner=${first.owner.id}&hole=8&view=hole`
  await page.goto(explicit)
  await expect(page.locator('#current-hole-heading')).toHaveText('8')
  await expect(page.getByRole('combobox', { name: 'Turnering', exact: true })).not.toBeVisible()
  const entry = await page.locator('.hole-entry').boundingBox(); const controls = await page.locator('.score-hole-controls').boundingBox()
  expect(entry && controls && entry.y < controls.y).toBe(true)
  await layout(page, 'long-populated')
  await page.locator('.score-context summary').focus()
  await page.keyboard.press('Enter')
  await expect(page.getByRole('combobox', { name: 'Turnering', exact: true })).toBeVisible()
  await layout(page, 'expanded')
  await page.locator('.score-context summary').click()
  const nav = page.getByRole('navigation', { name: 'Hovedmeny' })
  await nav.getByRole('link', { name: 'Profil' }).click()
  await nav.getByRole('link', { name: 'Score', exact: true }).click()
  await expect(page.locator('#current-hole-heading')).toHaveText('2')
  await expect(page).toHaveURL(new RegExp(`round=${round.id}`))
  await page.getByRole('combobox', { name: 'Hull', exact: true }).selectOption('7')
  await page.goBack()
  await expect(page.locator('#current-hole-heading')).toHaveText('2')
  await page.goForward()
  await expect(page.locator('#current-hole-heading')).toHaveText('7')
  await page.locator('.writable-card-switcher button').nth(1).click()
  await expect(page.locator('#current-hole-heading')).toHaveText('7')
  await expect(page).toHaveURL(new RegExp(`owner=${second.owner.id}`))
  await nav.getByRole('link', { name: 'Profil' }).click()
  let release: () => void = () => undefined
  pending = new Promise<void>(resolve => { release = resolve })
  await nav.getByRole('link', { name: 'Score', exact: true }).click()
  await expect(page.getByText('Laster …')).toBeVisible()
  await layout(page, 'loading')
  fail = true; release(); pending = null
  await expect(page.getByRole('alert')).toContainText('Midlertidig utilgjengelig')
  await layout(page, 'error')
  fail = false; scored = []
  await page.getByRole('button', { name: 'Prøv igjen', exact: true }).click()
  await expect(page.locator('#current-hole-heading')).toHaveText('1')
  await expect(page).toHaveURL(new RegExp(`owner=${second.owner.id}`))
  await page.route(`**/api/rounds/${round.id}/scores`, route => route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Lagring utilgjengelig' } } }))
  await page.getByRole('button', { name: /Registrer par/ }).click()
  await expect(page.locator('.score-save-error')).toBeVisible()
  await nav.getByRole('link', { name: 'Profil' }).click()
  await expect(page).toHaveURL(/\/score\?/)
  await layout(page, 'failed-save')
  await page.getByRole('button', { name: 'Forkast' }).click()
  await nav.getByRole('link', { name: 'Profil' }).click()
  scored = source.holes.map(hole => hole.hole_number)
  await nav.getByRole('link', { name: 'Score', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Bekreft fullført scorekort' })).toBeVisible()
  await expect(page).toHaveURL(/view=summary/)
  await layout(page, 'all-scored')
  await nav.getByRole('link', { name: 'Profil' }).click()
  empty = true
  await nav.getByRole('link', { name: 'Score', exact: true }).click()
  await expect(page.getByText('Runden har ingen kvalifiserte scorekort')).toBeVisible()
  await layout(page, 'empty')
  await nav.getByRole('link', { name: 'Turnering', exact: true }).click()
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    const create = await page.getByRole('link', { name: 'Opprett ny turnering' }).boundingBox()
    const filters = await page.locator('.tournament-list-views').boundingBox()
    expect(create && filters ? filters.y - create.y - create.height : 0).toBeGreaterThanOrEqual(16)
  }
  await page.unroute(`**/api/tournaments/${trip.id}/rounds`)
  await page.goto(`/leaderboard?tournament=${trip.id}&scope=tournament&metric=net`)
  await expect(page.getByRole('heading', { name: 'Netto resultat', exact: true })).toBeVisible()
  await expect(page.getByText(LIVE_RESULTS_EXPLANATION, { exact: true })).toHaveCount(0)
  await expect(page.getByText(MANDATORY_ROUND_EXPLANATION, { exact: true })).toHaveCount(0)
  expect(errors).toEqual([]); expect(failed).toEqual([])
})
