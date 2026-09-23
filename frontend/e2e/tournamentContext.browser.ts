import { test, expect } from '@playwright/test'
import { contextFixture } from './tournamentContextSupport'
import { decodeTournamentList } from '../src/api/tournaments/decoders'

test.skip(process.env.GOLF_CONTEXT_BROWSER !== '1', 'Requires disposable API and GOLF_CONTEXT_BROWSER=1.')

test('main results navigation preserves the non-default tournament and round', async ({ page }) => {
  const { first, second, list } = await contextFixture(page)
  const defaultId = decodeTournamentList(list).find(t => t.status === 'active')?.id
  const target = defaultId === second.id ? { id: first.tournament.id, round: first.round } : second
  await page.goto(`/rounds/${target.round.id}`)
  await expect(page.getByRole('heading', { name: target.round.name, exact: true })).toBeVisible()
  await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Resultater', exact: true }).click()
  await expect(page.getByRole('combobox', { name: 'Turnering', exact: true })).toHaveValue(target.id)
  await expect(page).toHaveURL(new RegExp(`tournament=${target.id}.*round=${target.round.id}`))
})

for (const width of [320, 390, 1280]) {
  test(`context survives score resume, profile, switching, history and reload at ${width}px`, async ({ page, browser }) => {
    const { first, second } = await contextFixture(page)
    await page.setViewportSize({ width, height: width === 320 ? 600 : 900 })
    const errors: string[] = [], failed: string[] = [], writes: string[] = []
    page.on('pageerror', error => errors.push(error.message))
    page.on('console', event => { if (event.type() === 'error') errors.push(event.text()) })
    page.on('requestfailed', request => { if (request.failure()?.errorText !== 'net::ERR_ABORTED') failed.push(request.failure()?.errorText ?? 'failed') })
    page.on('request', request => { if (request.method() === 'PUT' && new URL(request.url()).pathname.includes('/scores')) writes.push(new URL(request.url()).pathname) })
    const menu = page.getByRole('navigation', { name: 'Hovedmeny' })
    const score = menu.getByRole('link', { name: 'Score', exact: true })
    const results = menu.getByRole('link', { name: 'Resultater', exact: true })
    await page.goto(`/manage/tournaments/${second.id}?round=${second.round.id}#round-management`)
    await expect(score).toHaveAttribute('href', new RegExp(`tournament=${second.id}&round=${second.round.id}&resume=1`))
    await page.getByRole('link', { name: 'Tilbake til turneringen', exact: true }).click()
    await expect(score).toHaveAttribute('href', new RegExp(`tournament=${second.id}&round=${second.round.id}&resume=1`))
    await score.click()
    await expect(page.locator('#current-hole-heading')).toHaveText('1')
    await expect(page).toHaveURL(new RegExp(`tournament=${second.id}&round=${second.round.id}.*hole=1&view=hole`))
    await page.getByRole('button', { name: /Registrer par/ }).click()
    await expect(page.getByText('Lagret på serveren', { exact: true })).toBeVisible()
    await results.click()
    await expect(page.getByRole('combobox', { name: 'Turnering', exact: true })).toHaveValue(second.id)
    await page.getByRole('button', { name: 'Brutto', exact: true }).click()
    await menu.getByRole('link', { name: 'Profil', exact: true }).click()
    await expect(score).toHaveAttribute('href', new RegExp(`tournament=${second.id}&round=${second.round.id}`))
    await results.click()
    await expect(page).toHaveURL(new RegExp(`tournament=${second.id}.*metric=gross`))
    await score.click()
    await expect(page.locator('#current-hole-heading')).toHaveText('2')
    await page.reload()
    await expect(page.locator('#current-hole-heading')).toHaveText('2')
    await results.click()
    await page.goBack()
    await expect(page.locator('#current-hole-heading')).toHaveText('2')
    await page.goForward()
    await expect(page.getByRole('combobox', { name: 'Turnering', exact: true })).toHaveValue(second.id)
    await page.getByRole('combobox', { name: 'Turnering', exact: true }).selectOption(first.tournament.id)
    await expect(score).toHaveAttribute('href', new RegExp(`tournament=${first.tournament.id}&round=${first.round.id}`))
    await score.click()
    await expect(page.locator('#current-hole-heading')).toHaveText('1')
    await expect(page).toHaveURL(new RegExp(`tournament=${first.tournament.id}&round=${first.round.id}`))
    expect(writes.length).toBeGreaterThan(0)
    expect(writes.every(path => path.startsWith(`/api/rounds/${second.round.id}/scores`))).toBe(true)
    expect((await first.read()).holes_scored).toBe(0)
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    for (const link of await menu.getByRole('link').all()) {
      expect((await link.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      await link.click({ trial: true })
    }
    await page.screenshot({ path: `/tmp/golf-context-${width}.png`, fullPage: true })
    expect(errors).toEqual([])
    expect(failed).toEqual([])
    test.info().annotations.push({ type: 'Chrome', description: `${browser.version()} ${process.platform} ${width}px` })
  })
}

test('held access denial clears context; malformed routes keep escape navigation usable', async ({ page }) => {
  const { first } = await contextFixture(page)
  const accessPath = `**/api/rounds/${first.round.id}/score-access`
  let release: () => void = () => undefined
  const held = new Promise<void>(resolve => { release = resolve })
  await page.route(accessPath, async route => {
    await held
    await route.fulfill({ status: 403, json: { error: { code: 'forbidden', message: 'Context access denied' } } })
  })
  await page.goto(first.url())
  const menu = page.getByRole('navigation', { name: 'Hovedmeny' })
  const score = menu.getByRole('link', { name: 'Score', exact: true })
  await expect(score).toHaveAttribute('href', new RegExp(`tournament=${first.tournament.id}`))
  release()
  await expect(page.getByText('Context access denied', { exact: true })).toBeVisible()
  await expect(score).toHaveAttribute('href', '/score')
  await menu.getByRole('link', { name: 'Profil', exact: true }).click()
  await expect(score).toHaveAttribute('href', '/score')
  for (const path of ['/rounds/invalid', '/rounds/invalid/matches/invalid', `/tournaments/${first.tournament.id}/rounds/${first.round.id}/scorecards/invalid/${first.owner.id}`, '/profile/', '/tournaments/']) {
    await page.goto(path)
    await expect(score).not.toHaveAttribute('aria-disabled', 'true')
    await expect(menu.getByRole('link', { name: 'Resultater', exact: true })).not.toHaveAttribute('aria-disabled', 'true')
  }
})

test('a late round response cannot replace a newly selected tournament', async ({ page }) => {
  const { first, second } = await contextFixture(page)
  let release: () => void = () => undefined, requested: () => void = () => undefined
  const held = new Promise<void>(resolve => { release = resolve })
  const started = new Promise<void>(resolve => { requested = resolve })
  const handled: Promise<void>[] = []
  await page.route(`**/api/rounds/${first.round.id}`, route => {
    const response = (async () => { const result = await route.fetch(); requested(); await held; await route.fulfill({ response: result }) })()
    handled.push(response)
    return response
  })
  await page.goto(`/rounds/${first.round.id}`)
  await started
  const menu = page.getByRole('navigation', { name: 'Hovedmeny' })
  const score = menu.getByRole('link', { name: 'Score', exact: true })
  await expect(score).toHaveAttribute('aria-disabled', 'true')
  await menu.getByRole('link', { name: 'Turnering', exact: true }).click()
  await page.locator(`a[href="/tournaments/${second.id}"]`).click()
  await expect(score).toHaveAttribute('href', `/score?tournament=${second.id}&resume=1`)
  release()
  await Promise.all(handled)
  await menu.getByRole('link', { name: 'Profil', exact: true }).click()
  await expect(score).toHaveAttribute('href', `/score?tournament=${second.id}&resume=1`)
})

test('explicit tournament wins over a foreign round and login return preserves an exact card URL', async ({ page }) => {
  const { first, second } = await contextFixture(page)
  const preferred = second.rounds[1]
  if (!preferred) throw new Error('Missing preferred round')
  const foreignReads: string[] = []
  page.on('request', request => {
    if (new URL(request.url()).pathname.startsWith(`/api/rounds/${first.round.id}`)) foreignReads.push(request.url())
  })
  await page.goto(`/leaderboard?tournament=${second.id}&round=${first.round.id}&scope=round&metric=net`)
  await expect(page).toHaveURL(new RegExp(`tournament=${second.id}.*round=${preferred.id}`))
  const score = page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link', { name: 'Score', exact: true })
  await expect(score).toHaveAttribute('href', new RegExp(`tournament=${second.id}&round=${preferred.id}`))
  await score.click()
  await expect(page.locator('#current-hole-heading')).toHaveText('1')
  expect(foreignReads).toEqual([])
  await page.context().clearCookies()
  const explicit = first.url(7)
  await page.goto(explicit)
  await expect(page).toHaveURL(/\/login\?returnTo=/)
  await page.getByLabel('Brukernavn', { exact: true }).fill(first.username)
  await page.getByLabel('Passord', { exact: true }).fill(first.password)
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).toHaveURL(explicit)
  await expect(page.locator('#current-hole-heading')).toHaveText('7')
})

test('private history and read cards publish only their validated tournament and round', async ({ page }) => {
  const { first } = await contextFixture(page)
  await first.save(1, 4)
  const menu = page.getByRole('navigation', { name: 'Hovedmeny' })
  const results = menu.getByRole('link', { name: 'Resultater', exact: true })
  await page.goto(`/tournaments/${first.tournament.id}/rounds/${first.round.id}/scorecards/player/${first.owner.id}?metric=gross&view=summary`)
  await expect(results).toHaveAttribute('href', new RegExp(`tournament=${first.tournament.id}.*round=${first.round.id}&metric=gross`))
  await results.click()
  await expect(page.getByRole('combobox', { name: 'Turnering', exact: true })).toHaveValue(first.tournament.id)
  await page.goto(`/tournaments/${first.tournament.id}/results/players/${first.owner.id}?metric=gross`)
  await expect(results).toHaveAttribute('href', new RegExp(`tournament=${first.tournament.id}&scope=tournament.*metric=gross`))
  await menu.getByRole('link', { name: 'Score', exact: true }).click()
  await expect(page.locator('#current-hole-heading')).toHaveText('2')
})

test('match scoring denial clears navigation after successful round metadata', async ({ page, browser }) => {
  const { matchFixture } = await import('./matchSupport')
  const fixture = await matchFixture(page, browser)
  let release: () => void = () => undefined
  const held = new Promise<void>(resolve => { release = resolve })
  await page.route(`**/api/rounds/${fixture.round.id}/match-play/matches/${fixture.matchId}/scoring`, async route => {
    await held
    await route.fulfill({ status: 403, json: { error: { code: 'forbidden', message: 'Match context denied' } } })
  })
  await page.goto(fixture.url)
  const menu = page.getByRole('navigation', { name: 'Hovedmeny' })
  const score = menu.getByRole('link', { name: 'Score', exact: true })
  await expect(score).toHaveAttribute('href', new RegExp(`tournament=${fixture.tournament.id}&round=${fixture.round.id}`))
  release()
  await expect(page.getByText('Match context denied', { exact: true })).toBeVisible()
  await expect(score).toHaveAttribute('href', '/score')
  await menu.getByRole('link', { name: 'Profil', exact: true }).click()
  await expect(score).toHaveAttribute('href', '/score')
})

test('a real nonmember cannot establish context from another account’s tournament', async ({ page, browser }) => {
  const { first } = await contextFixture(page)
  const { offlineFixture } = await import('./offlineSupport')
  const other = await browser.newContext({ baseURL: 'http://127.0.0.1:5173' })
  try {
    const outsider = await offlineFixture(await other.newPage())
    await page.goto(first.url())
    await expect(page.locator('#current-hole-heading')).toHaveText('1')
    await page.goto(`/tournaments/${outsider.tournament.id}`)
    await expect(page.locator('.main-content').getByRole('alert')).toBeVisible()
    await expect(page.locator('.main-content').getByText(outsider.tournament.name, { exact: true })).toHaveCount(0)
    const menu = page.getByRole('navigation', { name: 'Hovedmeny' })
    await expect(menu.getByRole('link', { name: 'Score', exact: true })).toHaveAttribute('href', '/score')
    await menu.getByRole('link', { name: 'Profil', exact: true }).click()
    await expect(menu.getByRole('link', { name: 'Resultater', exact: true })).toHaveAttribute('href', '/leaderboard')
  } finally { await other.close() }
})
