import { test, expect, type Page } from '@playwright/test'
import { matchFixture } from './matchSupport'
import { decodeObject, decodeUuid } from '../src/api/decoder'
import { decodeTournamentRounds } from '../src/api/tournaments/decoders'

test.skip(process.env.GOLF_L3_BROWSER !== '1', 'Requires disposable API and GOLF_L3_BROWSER=1.')

async function mixedTrip(page: Page, csrf: string) {
  const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
  const result = await page.request.post('/api/tournaments', { headers: { 'x-csrf-token': csrf }, data: {
    request_id: crypto.randomUUID(),
    tournament: { name: `Blandet turnering med et langt navn for både matchspill og sammenlagtresultater ${Date.now()}`, description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
    rounds: [{ round_number: 1, name: 'Slagspill', round_date: day, scoring_format: 'individual_stroke_play' }, { round_number: 2, name: 'Matchspill', round_date: day, scoring_format: 'singles_match_play' }],
  } })
  expect(result.status(), await result.text()).toBe(201)
  const id = decodeUuid(decodeObject(await result.json(), 'receipt').tournament_id, 'tournament_id')
  const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${id}/rounds`)).json(), id)
  const stroke = rounds[0]
  if (!stroke) throw new Error('Missing stroke round')
  return { id, stroke }
}
function observe(page: Page, matchTrip: string, matchRound: string) {
  const errors: string[] = [], failures: string[] = [], statuses: number[] = [], forbiddenReads: string[] = []
  page.on('pageerror', error => errors.push(error.message))
  page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text()) })
  page.on('requestfailed', request => { if (request.failure()?.errorText !== 'net::ERR_ABORTED') failures.push(request.failure()?.errorText ?? 'failed') })
  page.on('response', response => { if (response.status() >= 400) statuses.push(response.status()) })
  page.on('request', request => {
    const path = new URL(request.url()).pathname
    if (path.startsWith(`/api/tournaments/${matchTrip}/leaderboards/`) || path.startsWith(`/api/rounds/${matchRound}/leaderboards/`)) forbiddenReads.push(path)
  })
  return { errors, failures, statuses, forbiddenReads }
}
async function inspect(page: Page, name: string) {
  const selector = page.getByRole('combobox', { name: 'Turnering', exact: true })
  await selector.scrollIntoViewIfNeeded()
  expect((await selector.boundingBox())?.height).toBeGreaterThanOrEqual(44)
  await selector.click({ trial: true })
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  await page.screenshot({ path: `/tmp/l3-${name}.png` })
}

test('global results switch between match-only and mixed trips with direct entry and browser history', async ({ page, browser }) => {
  const match = await matchFixture(page, browser), mixed = await mixedTrip(page, match.auth.csrf_token)
  const observed = observe(page, match.tournament.id, match.round.id)
  const original = `/leaderboard?tournament=${match.tournament.id}&scope=round&round=${match.round.id}&metric=gross&player=${match.firstId}`
  for (const [width, height] of [[320, 600], [390, 844], [1280, 900]] as const) {
    await page.setViewportSize({ width, height })
    await page.goto(original)
    const select = page.getByRole('combobox', { name: 'Turnering', exact: true })
    await expect(select).toHaveValue(match.tournament.id)
    await expect(page.locator('.match-table li')).toHaveCount(1)
    await expect(page.locator('.match-table')).toContainText(match.firstName)
    await expect(page.getByRole('group', { name: 'Beregning' })).toHaveCount(0)
    await inspect(page, `match-${width}`)
    await select.selectOption(mixed.id)
    await expect(page).toHaveURL(`/leaderboard?tournament=${mixed.id}&scope=round&round=${mixed.stroke.id}&metric=gross`)
    await expect(page.getByText('Runden er fortsatt en kladd. Resultater vises når runden åpnes.')).toBeVisible()
    await expect(page.getByRole('link', { name: 'Matchpoeng og historikk' })).toHaveAttribute('href', `/tournaments/${mixed.id}/match-results`)
    await inspect(page, `mixed-${width}`)
    await page.goBack()
    await expect(page).toHaveURL(original)
    await expect(select).toHaveValue(match.tournament.id)
    await expect(page.locator('.match-table li')).toHaveCount(1)
    await page.goForward()
    await expect(select).toHaveValue(mixed.id)
    await page.getByRole('button', { name: 'Turnering', exact: true }).click()
    await select.selectOption(match.tournament.id)
    await expect(page).toHaveURL(`/leaderboard?tournament=${match.tournament.id}&scope=tournament&metric=gross`)
    await expect(page.locator('.match-table li')).toHaveCount(2)
    await select.selectOption(mixed.id)
    await expect(page.getByRole('heading', { name: 'Brutto resultat', exact: true })).toBeVisible()
    await page.getByRole('link', { name: 'Matchpoeng og historikk' }).click()
    await expect(page.getByRole('combobox', { name: 'Turnering', exact: true })).toHaveCount(0)
    await page.getByRole('link', { name: 'Sammenlagt brutto/netto' }).click()
    await expect(select).toHaveValue(mixed.id)
    await expect(page.getByRole('heading', { name: 'Netto resultat', exact: true })).toBeVisible()
  }
  expect(observed).toEqual({ errors: [], failures: [], statuses: [], forbiddenReads: [] })
})

test('match loading, error and empty states retain a usable global tournament selector', async ({ page, browser }) => {
  const match = await matchFixture(page, browser), mixed = await mixedTrip(page, match.auth.csrf_token)
  const observed = observe(page, match.tournament.id, match.round.id)
  const path = `**/api/tournaments/${match.tournament.id}/match-table`
  for (const [width, height] of [[320, 600], [390, 844], [1280, 900]] as const) {
    await page.setViewportSize({ width, height })
    for (const mode of ['loading', 'error', 'empty'] as const) {
      let release: () => void = () => undefined
      const pending = new Promise<void>(resolve => { release = resolve })
      const handled: Promise<void>[] = []
      await page.route(path, route => {
        const response = (async () => {
        if (mode === 'loading') { await pending; return route.fulfill({ json: { tournament_id: match.tournament.id, entries: [] } }) }
        return route.fulfill(mode === 'error' ? { status: 500, json: { error: { code: 'test_failure', message: 'Matchpoeng kunne ikke lastes' } } } : { json: { tournament_id: match.tournament.id, entries: [] } })
        })()
        handled.push(response)
        return response
      })
      await page.goto(`/leaderboard?tournament=${match.tournament.id}`)
      const select = page.getByRole('combobox', { name: 'Turnering', exact: true })
      await expect(select).toHaveValue(match.tournament.id)
      if (mode === 'loading') await expect(page.getByRole('status').filter({ hasText: 'Laster …' }).first()).toBeVisible()
      if (mode === 'error') await expect(page.getByText('Matchpoeng kunne ikke lastes', { exact: true })).toBeVisible()
      if (mode === 'empty') await expect(page.getByText('Ingen spillere er registrert.', { exact: true })).toBeVisible()
      await inspect(page, `${mode}-${width}`)
      await select.selectOption(mixed.id)
      await expect(select).toHaveValue(mixed.id)
      await expect(page.getByRole('heading', { name: 'Resultater', exact: true })).toBeVisible()
      release()
      await Promise.all(handled)
      await page.unroute(path)
    }
  }
  expect(observed.errors).toEqual([])
  expect(observed.failures).toEqual([])
  expect(observed.statuses.every(status => status === 500)).toBe(true)
  expect(observed.forbiddenReads).toEqual([])
})
