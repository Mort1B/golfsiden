import { test, expect, type Page } from '@playwright/test'
import { decodeTournament } from '../src/api/tournaments/decoders'
import { decodeAuthSession } from '../src/api/auth'
import { decodeObject, decodeString, decodeUuid } from '../src/api/decoder'

test.skip(process.env.GOLF_RECOVERY_BROWSER !== '1', 'Requires disposable recovery API and GOLF_RECOVERY_BROWSER=1.')
test.use({ screenshot: 'off', trace: 'off' })

async function layout(page: Page, name: string) {
  for (const [width, height] of [[320, 600], [390, 844], [1280, 900]] as const) {
    await page.setViewportSize({ width, height })
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true)
    for (const control of await page.locator('.player-recovery button:visible, .recovery-page button:visible, .recovery-page a:visible').all()) {
      expect((await control.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await control.isEnabled()) {
        await control.evaluate(element => element.scrollIntoView({ block: 'center' }))
        await control.click({ trial: true })
      }
    }
    await page.screenshot({ path: `/tmp/golf-recovery-${name}-${width}.png`, fullPage: true, mask: [page.getByLabel('Lenke for nytt passord', { exact: true })] })
  }
}
function observe(page: Page) {
  const errors: string[] = []
  const failures: Array<{ path: string; status: number }> = []
  const networkFailures: Array<{ path: string; error: string | undefined }> = []
  page.on('pageerror', () => errors.push('pageerror'))
  page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push('console error') })
  page.on('response', response => { if (response.status() >= 400) failures.push({ path: new URL(response.url()).pathname, status: response.status() }) })
  page.on('requestfailed', request => networkFailures.push({ path: new URL(request.url()).pathname, error: request.failure()?.errorText }))
  return { errors, failures, networkFailures }
}

test('real administrator issue/copy/redeem/relogin and lost-link revoke preserve unrelated session', async ({ page, browser }) => {
  const events = observe(page)
  const unique = Date.now()
  const password = 'browser-recovery-initial'
  const newPassword = 'browser-recovery-replaced'
  const playerName = 'Spiller med et svært langt etternavn som trenger hjelp med glemt passord'
  const username = `recover_${unique}`
  const date = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
  const created = await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username: `organizer_${unique}`, password }, player: { display_name: 'Arrangør', handicap_index: 8 } },
    tournament: { name: 'Passordhjelp for golfturnering', description: '', start_date: date, end_date: date, counted_rounds: 1, mandatory_round_number: null },
    rounds: [{ round_number: 1, name: 'Første runde', round_date: date, scoring_format: 'individual_stroke_play' }],
  } })
  expect(created.status()).toBe(201)
  const body = decodeObject(await created.json(), 'onboarding')
  const invitation = decodeObject(body.invitation, 'onboarding.invitation')
  const trip = { tournament: decodeTournament(body.tournament), session: decodeAuthSession(body.session),
    invitation: { id: decodeUuid(invitation.id, 'invitation.id'), token: decodeString(invitation.token, 'invitation.token') } }
  const playerContext = await browser.newContext()
  try {
    const player = await playerContext.newPage()
    const joined = await player.request.post(`http://127.0.0.1:5173/api/invitations/${trip.invitation.id}/register`, { data: {
      token: trip.invitation.token, account: { username, password }, player: { display_name: playerName, handicap_index: 12 },
    } })
    expect(joined.status()).toBe(201)
    const registration = decodeObject(await joined.json(), 'joined')
    const playerId = decodeUuid(registration.player_id, 'joined.player_id')
    await page.goto(`/tournaments/${trip.tournament.id}`)
    await page.getByRole('button', { name: `Hjelp med glemt passord for ${playerName}` }).click()
    await layout(page, 'admin-longname')
    await page.getByLabel('Ditt nåværende passord').fill('incorrect-password')
    await page.getByRole('button', { name: 'Lag lenke for nytt passord' }).click()
    await expect(page.getByRole('alert')).toContainText('passord er ikke riktig')
    await layout(page, 'admin-wrong-password')
    await page.getByLabel('Ditt nåværende passord').fill(password)
    await page.getByRole('button', { name: 'Lag lenke for nytt passord' }).click()
    await expect(page.getByLabel('Lenke for nytt passord', { exact: true })).toBeVisible()
    const resetUrl = await page.getByLabel('Lenke for nytt passord', { exact: true }).inputValue()
    await page.context().grantPermissions(['clipboard-read', 'clipboard-write'])
    await page.getByRole('button', { name: 'Kopier lenke', exact: true }).click()
    await expect(page.getByText('Lenken er kopiert.')).toBeVisible()
    expect(await page.evaluate(async url => (await navigator.clipboard.readText()) === url, resetUrl)).toBe(true)
    await page.evaluate(() => { Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async () => { throw new Error('clipboard unavailable') } } }) })
    await page.getByRole('button', { name: 'Kopier lenke', exact: true }).click()
    await expect(page.getByText(/Kunne ikke kopiere/)).toBeVisible()
    await layout(page, 'admin-receipt')
    // Use the organizer's signed-in browser as recipient: recovery must retain
    // this unrelated account and still offer the normal sign-in form.
    await page.goto(resetUrl)
    await expect(page.getByLabel('Nytt passord', { exact: true })).toBeVisible()
    expect(new URL(page.url()).hash === '').toBe(true)
    await layout(page, 'public-form')
    await page.getByLabel('Nytt passord', { exact: true }).fill(newPassword)
    await page.getByLabel('Gjenta nytt passord').fill(newPassword)
    await page.getByRole('button', { name: 'Lagre nytt passord' }).click()
    await expect(page.getByText(/Kontoens tidligere økter er avsluttet/)).toBeVisible()
    await layout(page, 'public-success')
    expect(decodeAuthSession(await (await page.request.get('/api/auth/session')).json()).user_id).toBe(trip.session.user_id)
    expect((await player.request.get('http://127.0.0.1:5173/api/auth/session')).status()).toBe(401)
    await page.getByRole('link', { name: 'Gå til innlogging' }).click()
    await expect(page.getByLabel('Brukernavn', { exact: true })).toBeVisible()
    await player.goto('/login')
    await player.getByLabel('Brukernavn', { exact: true }).fill(username)
    await player.getByLabel('Passord', { exact: true }).fill(newPassword)
    await player.getByRole('button', { name: 'Logg inn', exact: true }).click()
    await expect(player).not.toHaveURL(/\/login/)
    // Issue another grant; close the receipt then revoke without its identifier.
    await page.goto(`/tournaments/${trip.tournament.id}`)
    await page.getByRole('button', { name: `Hjelp med glemt passord for ${playerName}` }).click()
    await page.getByLabel('Ditt nåværende passord').fill(password)
    await page.getByRole('button', { name: 'Lag lenke for nytt passord' }).click()
    await expect(page.getByLabel('Lenke for nytt passord', { exact: true })).toBeVisible()
    const revokedUrl = await page.getByLabel('Lenke for nytt passord', { exact: true }).inputValue()
    await page.getByRole('button', { name: /Lukk passordhjelp/ }).click()
    await page.getByRole('button', { name: /Hjelp med glemt passord/ }).click()
    await page.getByLabel('Ditt nåværende passord').fill(password)
    await page.getByRole('button', { name: 'Tilbakekall lenker' }).click()
    await expect(page.getByText(/Utestående lenker/)).toBeVisible()
    await layout(page, 'admin-revoked')
    await page.goto(revokedUrl)
    await expect(page.getByRole('alert')).toContainText('Lenken er ugyldig')
    await layout(page, 'public-revoked')
    const fresh = await page.request.post(`/api/tournaments/${trip.tournament.id}/players/${playerId}/password-recovery`, {
      headers: { 'x-csrf-token': trip.session.csrf_token }, data: { current_password: password },
    })
    expect(fresh.status()).toBe(201)
    const loggedOutUrl = decodeString(decodeObject(await fresh.json(), 'recovery').reset_url, 'recovery.reset_url')
    await playerContext.clearCookies()
    await player.goto(loggedOutUrl)
    await expect(player.getByLabel('Nytt passord', { exact: true })).toBeVisible()
    await player.getByLabel('Nytt passord', { exact: true }).fill('final recovered password')
    await player.getByLabel('Gjenta nytt passord').fill('final recovered password')
    await player.getByRole('button', { name: 'Lagre nytt passord' }).click()
    await expect(player.getByText(/Kontoens tidligere økter er avsluttet/)).toBeVisible()
    expect((await player.request.get('http://127.0.0.1:5173/api/auth/session')).status()).toBe(401)
    await layout(player, 'logged-out-success')
    await player.goto(loggedOutUrl)
    await expect(player.getByRole('alert')).toContainText('Lenken er ugyldig')
    expect(events.errors).toEqual([])
    expect(events.networkFailures.every(({ path, error }) => error === 'net::ERR_ABORTED' && (path.endsWith('/live') || path.endsWith('/preview') || path.endsWith('/redeem') || path.endsWith('/revoke')))).toBe(true)
    expect(events.failures.every(({ status, path }) => status === 409 && (path.endsWith('/preview') || path === `/api/tournaments/${trip.tournament.id}/players/${playerId}/password-recovery`))).toBe(true)
  } finally { await playerContext.close() }
})

test('public loading, retry, missing and expired states on mobile and desktop', async ({ page }) => {
  const events = observe(page)
  const id = '00000000-0000-0000-0000-000000000077'
  const token = 'a'.repeat(43)
  let finish: () => void = () => { throw new Error('missing pending request') }
  let mode = 'loading'
  await page.route(`**/api/auth/password-recovery/${id}/preview`, async route => {
    if (mode === 'loading') await new Promise<void>(resolve => { finish = resolve })
    if (mode === 'error') return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'private internal detail' } } })
    if (mode === 'invalid') return route.fulfill({ status: 409, json: { error: { code: 'password_recovery_invalid', message: 'private internal detail' } } })
    return route.fulfill({ json: { id, expires_at: '2099-01-01T12:00:00Z' } })
  })
  await page.goto(`/reset-password/${id}#token=${token}`)
  await expect(page.getByText('Kontrollerer lenken …')).toBeVisible()
  await layout(page, 'public-loading')
  mode = 'error'; finish()
  await expect(page.getByRole('button', { name: 'Prøv igjen' })).toBeVisible()
  await layout(page, 'public-error')
  mode = 'ready'
  await page.getByRole('button', { name: 'Prøv igjen' }).click()
  await expect(page.getByLabel('Nytt passord', { exact: true })).toBeVisible()
  await page.getByLabel('Nytt passord', { exact: true }).fill('ø'.repeat(65))
  await page.getByLabel('Gjenta nytt passord').fill('ø'.repeat(65))
  await page.getByRole('button', { name: 'Lagre nytt passord' }).click()
  await expect(page.getByRole('alert')).toContainText('Grensen er 128 byte')
  await layout(page, 'public-validation')
  mode = 'invalid'
  await page.goto(`/reset-password/${id}#token=${token}`)
  await expect(page.getByRole('alert')).toContainText('Lenken er ugyldig')
  await page.goto(`/reset-password/${id}`)
  await expect(page.getByRole('alert')).toContainText('Lenken er ugyldig')
  await layout(page, 'public-missing')
  expect(events.errors).toEqual([])
  expect(events.networkFailures.every(({ path, error }) => error === 'net::ERR_ABORTED' && path.endsWith('/preview'))).toBe(true)
  expect(events.failures.every(({ status, path }) => (path === '/api/auth/session' && status === 401) || (path.endsWith('/preview') && [409, 503].includes(status)))).toBe(true)
})
