import { test, expect, type Page } from '@playwright/test'
import { decodeObject } from '../src/api/decoder'
import { decodeAuthSession } from '../src/api/auth'
import { decodeTournament } from '../src/api/tournaments/decoders'
import { decodeClaimReceipt, decodeClaimPreview, decodeClaimRegistration } from '../src/api/playerClaimDecoders'

test.skip(process.env.GOLF_PLAYER_CLAIMS_BROWSER !== '1', 'Requires disposable player claim API and GOLF_PLAYER_CLAIMS_BROWSER=1.')
test.use({ screenshot: 'off', trace: 'off' })
async function layout(page: Page, name: string) {
  for (const [width, height] of [[320, 600], [390, 844], [1280, 900]] as const) {
    await page.setViewportSize({ width, height })
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    for (const button of await page.locator('.player-management button:visible, .claim-page button:visible, .claim-page a:visible').all()) {
      expect((await button.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await button.isEnabled()) { await button.scrollIntoViewIfNeeded(); await button.click({ trial: true }) }
    }
    await page.screenshot({ path: `/tmp/golf-player-claims-20261008/${name}-${width}.png`, fullPage: true, mask: [page.getByLabel('Personlig kontolenke'), page.locator('input[type=password]')] })
  }
}
function observe(page: Page) {
  const errors: string[] = []
  const failures: Array<{ path: string; status: number }> = []
  const failed: Array<{ path: string; error: string | undefined }> = []
  page.on('pageerror', () => errors.push('pageerror'))
  page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push('console error') })
  page.on('response', response => { if (response.status() >= 400) failures.push({ path: new URL(response.url()).pathname, status: response.status() }) })
  page.on('requestfailed', request => failed.push({ path: new URL(request.url()).pathname, error: request.failure()?.errorText }))
  return { errors, failures, failed }
}
async function setup(page: Page) {
  const unique = Date.now()
  const date = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
  const result = await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username: `claim_admin_${unique}`, password: 'player claims browser password' }, player: { display_name: 'Arrangør', handicap_index: 8 } },
    tournament: { name: 'Spillere og personlige kontolenker', description: '', start_date: date, end_date: date, counted_rounds: 1, mandatory_round_number: null },
    rounds: [{ round_number: 1, name: 'Første runde', round_date: date, scoring_format: 'individual_stroke_play' }],
  } })
  expect(result.status()).toBe(201)
  const data = decodeObject(await result.json(), 'onboarding')
  return { tournament: decodeTournament(data.tournament), session: decodeAuthSession(data.session), unique }
}
function secretParts(url: string) {
  const parsed = new URL(url)
  return { id: parsed.pathname.split('/').at(-1) ?? '', token: new URLSearchParams(parsed.hash.slice(1)).get('token') ?? '' }
}
test('real create, copy, exact identity claim, replay, reissue, revoke and withdrawal', async ({ page, browser }) => {
  const events = observe(page)
  const trip = await setup(page)
  const longName = 'Spiller med et langt navn som skal få sin egen konto og beholde riktig identitet'
  await page.goto(`/tournaments/${trip.tournament.id}`)
  await page.getByRole('link', { name: 'Administrer spillere' }).click()
  await page.getByLabel('Spillerens navn').fill(longName)
  await page.getByLabel('Spillerens handicap').fill('14,4')
  const response = page.waitForResponse(response => response.url().endsWith(`/api/tournaments/${trip.tournament.id}/players`) && response.request().method() === 'POST')
  await page.getByRole('button', { name: 'Opprett spiller og kontolenke' }).click()
  const receipt = decodeClaimReceipt(await (await response).json())
  await expect(page.getByLabel('Personlig kontolenke')).toBeVisible()
  const url = await page.getByLabel('Personlig kontolenke').inputValue()
  await page.context().grantPermissions(['clipboard-read', 'clipboard-write'])
  await page.getByRole('button', { name: 'Kopier lenke', exact: true }).click()
  await expect(page.getByText('Lenken er kopiert.')).toBeVisible()
  expect(await page.evaluate(async value => await navigator.clipboard.readText() === value, url)).toBe(true)
  await page.evaluate(() => Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async () => { throw new Error('unavailable') } } }))
  await page.getByRole('button', { name: 'Kopier lenke', exact: true }).click()
  await expect(page.getByText(/Kunne ikke kopiere/)).toBeVisible()
  await layout(page, 'admin-longname-receipt')
  const context = await browser.newContext()
  try {
    const recipient = await context.newPage()
    const recipientEvents = observe(recipient)
    await recipient.goto(url)
    await expect(recipient.getByLabel('Brukernavn', { exact: true })).toBeVisible()
    expect(new URL(recipient.url()).hash).toBe('')
    await expect(recipient.getByText(longName, { exact: true })).toBeVisible()
    await layout(recipient, 'claim-ready')
    await recipient.getByLabel('Brukernavn', { exact: true }).fill(`claimed_${trip.unique}`)
    await recipient.getByLabel('Passord', { exact: true }).fill('claimed account password')
    const registered = recipient.waitForResponse(response => response.url().endsWith('/register'))
    await recipient.getByRole('button', { name: 'Ta i bruk kontoen' }).click()
    await expect(recipient.getByText(/Kontoen er klar/)).toBeVisible()
    const preview = decodeClaimPreview({ tournament: { id: trip.tournament.id, name: trip.tournament.name }, player: { id: receipt.player_id, display_name: longName }, expires_at: receipt.expires_at })
    expect(decodeClaimRegistration(await (await registered).json(), preview).player_id).toBe(receipt.player_id)
    const actual = decodeAuthSession(await (await recipient.request.get('http://127.0.0.1:5173/api/auth/session')).json())
    expect(actual.player_id).toBe(receipt.player_id)
    await layout(recipient, 'claim-success')
    await recipient.goto(url)
    await expect(recipient.getByRole('alert')).toContainText('Lenken er ugyldig')
    await layout(recipient, 'claim-replay')
    await page.reload()
    const row = page.locator('.managed-players li').filter({ hasText: longName })
    await expect(row.getByText(/Konto tatt i bruk/)).toBeVisible()
    await expect(row.getByRole('button', { name: /Lag ny kontolenke/ })).toHaveCount(0)
    await row.getByRole('button', { name: /Fjern/ }).click()
    await expect(row.getByText(/Historiske resultater/)).toBeVisible()
    await row.getByRole('button', { name: 'Bekreft fjerning' }).click()
    await expect(row.getByText(/Trukket/)).toBeVisible()
    await layout(page, 'withdrawn')
    expect((await recipient.request.get(`http://127.0.0.1:5173/api/tournaments/${trip.tournament.id}`)).status()).toBe(200)
    await page.getByLabel('Spillerens navn').fill('Ny spiller for lenker')
    await page.getByLabel('Spillerens handicap').fill('9')
    await page.getByRole('button', { name: 'Opprett spiller og kontolenke' }).click()
    await expect(page.getByLabel('Personlig kontolenke')).toBeVisible()
    const firstUrl = await page.getByLabel('Personlig kontolenke').inputValue()
    await page.getByRole('button', { name: 'Lag ny kontolenke for Ny spiller for lenker' }).click()
    await expect(page.getByLabel('Personlig kontolenke')).not.toHaveValue(firstUrl)
    const nextUrl = await page.getByLabel('Personlig kontolenke').inputValue()
    const old = secretParts(firstUrl)
    expect((await recipient.request.post(`http://127.0.0.1:5173/api/player-claims/${old.id}/preview`, { data: { token: old.token } })).status()).toBe(410)
    await page.getByRole('button', { name: 'Skjul lenken' }).click()
    await page.getByRole('button', { name: 'Tilbakekall kontolenke for Ny spiller for lenker' }).click()
    await expect(page.getByText('Kontolenken er tilbakekalt.')).toBeVisible()
    await recipient.goto(nextUrl)
    await expect(recipient.getByRole('alert')).toContainText('Lenken er ugyldig')
    expect(recipientEvents.errors).toEqual([])
    expect(recipientEvents.failures.every(item => item.path === '/api/auth/session' && item.status === 401 || item.path.endsWith('/preview') && item.status === 410)).toBe(true)
    expect(recipientEvents.failed.every(item => item.error === 'net::ERR_ABORTED')).toBe(true)
  } finally { await context.close() }
  expect(events.errors).toEqual([])
  expect(events.failures).toEqual([])
  expect(events.failed.every(item => item.error === 'net::ERR_ABORTED')).toBe(true)
})

test('public loading, retry, missing and unavailable states at mobile and desktop', async ({ page }) => {
  const events = observe(page)
  const id = '00000000-0000-0000-0000-000000000077'
  let finish: () => void = () => undefined
  let mode = 'loading'
  await page.route(`**/api/player-claims/${id}/preview`, async route => {
    if (mode === 'loading') await new Promise<void>(resolve => { finish = resolve })
    if (mode === 'error') return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'internal' } } })
    return route.fulfill({ json: { tournament: { id, name: 'Turnering med langt navn' }, player: { id, display_name: 'Forberedt spiller' }, expires_at: '2099-01-01T12:00:00Z' } })
  })
  await page.goto(`/claim/${id}#token=${'a'.repeat(43)}`)
  await expect(page.getByText('Kontrollerer lenken …')).toBeVisible()
  await layout(page, 'public-loading')
  mode = 'error'; finish()
  await expect(page.getByRole('button', { name: 'Prøv igjen' })).toBeVisible()
  await layout(page, 'public-error')
  mode = 'ready'; await page.getByRole('button', { name: 'Prøv igjen' }).click()
  await expect(page.getByLabel('Brukernavn')).toBeVisible()
  await page.goto(`/claim/${id}`)
  await expect(page.getByRole('alert')).toContainText('Lenken er ugyldig')
  await layout(page, 'public-missing')
  expect(events.errors).toEqual([])
  expect(events.failures.every(item => item.path === '/api/auth/session' && item.status === 401 || item.path.endsWith('/preview') && item.status === 503)).toBe(true)
})

test('admin roster empty, loading, error and restored populated states', async ({ page }) => {
  const events = observe(page)
  const trip = await setup(page)
  let mode = 'empty'
  let finish: () => void = () => undefined
  await page.route(`**/api/tournaments/${trip.tournament.id}/players`, async route => {
    if (route.request().method() !== 'GET') return route.continue()
    const response = await route.fetch()
    if (mode === 'empty') {
      const data = decodeObject(await response.json(), 'roster')
      return route.fulfill({ json: { ...data, players: [] } })
    }
    return route.fulfill({ response })
  })
  await page.route(`**/api/tournaments/${trip.tournament.id}/player-accounts`, async route => {
    if (mode === 'loading') await new Promise<void>(resolve => { finish = resolve })
    if (mode === 'error') return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'internal' } } })
    return route.continue()
  })
  await page.goto(`/manage/tournaments/${trip.tournament.id}#entrants`)
  await expect(page.getByText('Ingen deltakere er registrert.')).toBeVisible()
  await layout(page, 'admin-empty')
  mode = 'loading'; await page.reload()
  await expect(page.getByText('Laster kontostatus …')).toBeVisible()
  await layout(page, 'admin-loading')
  mode = 'error'; finish()
  await expect(page.getByText('Kunne ikke laste kontostatus.')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Opprett spiller og kontolenke' })).toBeDisabled()
  await layout(page, 'admin-error')
  mode = 'ready'
  await page.locator('.player-management').getByRole('button', { name: 'Prøv igjen' }).click()
  await expect(page.getByRole('button', { name: 'Opprett spiller og kontolenke' })).toBeEnabled()
  await expect(page.locator('.managed-players').getByText('Arrangør')).toBeVisible()
  expect(events.errors).toEqual([])
  expect(events.failures.every(item => item.path.endsWith('/player-accounts') && item.status === 503)).toBe(true)
  expect(events.failed.every(item => item.error === 'net::ERR_ABORTED')).toBe(true)
})
