import { test, expect, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeProfile } from '../src/api/profile'
import { decodeObject, decodeString, decodeUuid } from '../src/api/decoder'
import { decodeMyTournaments, decodeTournament, decodeTournamentPlayerRoster } from '../src/api/tournaments/decoders'
test.skip(process.env.GOLF_PROFILE_BROWSER !== '1', 'Requires disposable database and GOLF_PROFILE_BROWSER=1.')

async function layout(page: Page, state: string) {
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const button of await page.locator('.profile-page button:visible').all()) {
      expect((await button.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await button.isEnabled()) await button.click({ trial: true })
    }
    for (const link of await page.getByRole('navigation', { name: 'Hovedmeny' }).getByRole('link').all()) {
      expect((await link.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      await link.click({ trial: true })
    }
    await page.screenshot({ path: `/tmp/golf-profile-${state}-${width}.png`, fullPage: true })
  }
}
async function login(page: Page, username: string, password: string) {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill(username)
  await page.getByLabel('Passord', { exact: true }).fill(password)
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
}

test('profile edits, preserved trip handicap, duplicate/stale failures and all-device password logout', async ({ page, browser }) => {
  const username = `profile_${Date.now()}`
  const password = 'browser-profile-password'
  const tomorrow = new Date(Date.now() + 86_400_000).toISOString().slice(0, 10)
  const created = await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username, password }, player: { display_name: 'Profilspiller', handicap_index: 8.2 } },
    tournament: { name: `En turnering med et svært langt navn for å kontrollere mobilvisningen ${Date.now()}`, description: '', start_date: tomorrow, end_date: tomorrow, counted_rounds: 1, mandatory_round_number: null },
    rounds: [{ round_number: 1, name: 'Første runde', round_date: tomorrow, scoring_format: 'individual_stroke_play' }],
  } })
  expect(created.status()).toBe(201)
  const onboarding = decodeObject(await created.json(), 'onboarding')
  const trip = { tournament: decodeTournament(onboarding.tournament), session: decodeAuthSession(onboarding.session) }
  const second = await browser.newContext()
  try {
    const secondPage = await second.newPage()
    await login(secondPage, username, password)
    const errors: string[] = []; const failed: Array<{ path: string; status: number }> = []
    const requests: Array<Promise<{ path: string; error: string | undefined; status: number | undefined }>> = []
    page.on('pageerror', (e) => errors.push(e.message))
    page.on('console', (m) => { if (m.type() === 'error' && !m.text().startsWith('Failed to load resource:')) errors.push(m.text()) })
    page.on('response', (r) => { if (r.status() >= 400) failed.push({ path: new URL(r.url()).pathname, status: r.status() }) })
    page.on('requestfailed', (r) => requests.push((async () => ({ path: new URL(r.url()).pathname,
      error: r.failure()?.errorText, status: (await r.response())?.status() }))()))
    await page.goto('/tournaments')
    await page.getByRole('link', { name: 'Profil', exact: true }).click()
    await expect(page.getByRole('heading', { name: 'Min profil' })).toBeVisible()
    const summaries = page.locator('.profile-disclosure summary')
    await expect(page.locator('.profile-disclosure[open]')).toHaveCount(0)
    expect(await page.locator('.profile-page > header + section').getAttribute('aria-labelledby')).toBe('profile-tournaments')
    await summaries.first().focus()
    await page.keyboard.press('Enter')
    await expect(page.getByRole('button', { name: 'Lagre brukernavn' })).toBeVisible()
    await page.keyboard.press('Space')
    await expect(page.getByRole('button', { name: 'Lagre brukernavn' })).toBeHidden()
    await layout(page, 'populated')
    const longName = 'Et langt spillernavn med flere mellomnavn og et etternavn som fortsatt skal være leselig på mobil'
    await page.getByLabel('Navn', { exact: true }).fill(longName)
    await page.getByLabel('Handicap', { exact: true }).fill('14,4')
    await expect(page.getByLabel('Begrunnelse for handicapendring')).toHaveCount(0)
    await page.getByRole('button', { name: 'Lagre navn og handicap' }).click()
    await expect(page.getByText('Endringen er lagret og profilen er oppdatert.')).toBeVisible()
    await expect(page.getByLabel('Handicap', { exact: true })).toHaveValue('14,4')
    await expect(page.locator('.session-bar')).toContainText(longName)
    await layout(page, 'long-name')
    const roster = decodeTournamentPlayerRoster(await (await page.request.get(`/api/tournaments/${trip.tournament.id}/players`)).json(), trip.tournament.id)
    expect(roster.players.find(p => p.player_id === trip.session.player_id)?.tournament_handicap).toBe(8.2)
    const usernameForm = page.locator('form').filter({ has: page.getByRole('heading', { name: 'Endre brukernavn' }) })
    await usernameForm.locator('summary').click()
    await usernameForm.getByLabel('Brukernavn', { exact: true }).fill('admin')
    await usernameForm.getByLabel('Nåværende passord').fill(password)
    await usernameForm.getByRole('button').click()
    await expect(page.getByRole('alert')).toContainText('Brukernavnet er opptatt')
    await layout(page, 'duplicate')
    await usernameForm.getByLabel('Brukernavn', { exact: true }).fill(`${username}_new`)
    await usernameForm.getByLabel('Nåværende passord').fill('wrong-password')
    await usernameForm.getByRole('button').click()
    await expect(page.getByRole('alert')).toContainText('Nåværende passord er ikke riktig')
    await usernameForm.getByLabel('Nåværende passord').fill(password)
    await usernameForm.getByRole('button').click()
    await expect(page.getByText('Endringen er lagret og profilen er oppdatert.')).toBeVisible()
    await usernameForm.locator('summary').click()
    await expect(usernameForm.getByLabel('Brukernavn', { exact: true })).toHaveValue(`${username}_new`)
    // A second device edits after the first device's version was loaded.
    const otherSession = decodeAuthSession(await (await secondPage.request.get('/api/auth/session')).json())
    const old = decodeProfile(await (await secondPage.request.get('/api/me/profile')).json(), otherSession.user_id)
    const changed = await secondPage.request.put('/api/me/profile', { headers: { 'x-csrf-token': otherSession.csrf_token }, data: {
      version: old.version, player_updated_at: old.player_updated_at, display_name: 'Navn fra en annen enhet', handicap: old.handicap,
    } })
    expect(changed.ok()).toBe(true)
    await page.getByRole('button', { name: 'Lagre navn og handicap' }).click()
    await expect(page.getByRole('alert')).toContainText('Profilen er endret siden')
    await page.getByRole('button', { name: 'Oppdater profil og turneringer' }).click()
    await expect(page.getByLabel('Navn', { exact: true })).toHaveValue('Navn fra en annen enhet')
    const passwordForm = page.locator('form').filter({ has: page.getByRole('heading', { name: 'Endre passord', exact: true }) })
    await passwordForm.locator('summary').click()
    await passwordForm.getByLabel('Nytt passord', { exact: true }).fill('ø'.repeat(65))
    await passwordForm.getByLabel('Gjenta nytt passord').fill('ø'.repeat(65))
    await passwordForm.getByLabel('Nåværende passord').fill(password)
    await passwordForm.getByRole('button').click()
    await expect(passwordForm.getByRole('alert')).toContainText('Grensen er 128 byte')
    await passwordForm.locator('summary').click()
    await expect(passwordForm.getByRole('alert')).toBeVisible()
    await layout(page, 'password-overflow-collapsed')
    await passwordForm.locator('summary').click()
    await layout(page, 'password-expanded')
    await passwordForm.getByLabel('Nytt passord', { exact: true }).fill('browser-replaced-password')
    await passwordForm.getByLabel('Gjenta nytt passord').fill('browser-replaced-password')
    await passwordForm.getByLabel('Nåværende passord').fill(password)
    await passwordForm.getByRole('button').click()
    await expect(page).toHaveURL(/\/login/)
    await expect(page.getByRole('status')).toContainText('Passordet er endret')
    expect((await secondPage.request.get('/api/me/profile')).status()).toBe(401)
    await login(page, `${username}_new`, 'browser-replaced-password')
    await page.getByRole('link', { name: 'Profil', exact: true }).click()
    await expect(page.getByLabel('Navn', { exact: true })).toHaveValue('Navn fra en annen enhet')
    expect(errors).toEqual([])
    // Chrome reports aborted empty-body reads after already-confirmed 204s, and
    // closing a tournament route deliberately closes its SSE connection.
    for (const request of await Promise.all(requests)) {
      expect(request.error).toBe('net::ERR_ABORTED')
      expect(request.path.endsWith('/live') || (request.status === 204 && ['/api/me/profile/username', '/api/me/profile/password'].includes(request.path))).toBe(true)
    }
    expect(failed.filter(r => r.status !== 401).map(r => r.status)).toEqual([409, 409, 409])
    for (const response of failed.filter(r => r.status === 401)) expect(response.path).toBe('/api/auth/session')
  } finally { await second.close() }
})

test('loading, retry, empty, archived and inactive profile states remain accessible', async ({ page }) => {
  await login(page, 'admin', 'golf-dev-2026')
  const session = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
  const profile = decodeProfile(await (await page.request.get('/api/me/profile')).json(), session.user_id)
  const memberships = decodeMyTournaments(await (await page.request.get('/api/me/tournaments')).json())
  let finish: () => void = () => { throw new Error('No pending request') }
  let mode = 'loading'
  await page.route('**/api/me/profile', async route => {
    if (mode === 'loading') await new Promise<void>((resolve) => { finish = resolve })
    if (mode === 'error') return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Kunne ikke hente profilen.' } } })
    return route.fulfill({ json: mode === 'inactive' ? { ...profile, player_id: session.user_id, player_active: false, handicap: 9.1, player_updated_at: '2026-09-06T12:00:00Z' } : profile })
  })
  await page.route('**/api/me/tournaments', route => route.fulfill({ json: [] }))
  await page.goto('/profile')
  await expect(page.getByText('Laster …')).toBeVisible()
  await layout(page, 'loading')
  mode = 'error'; finish()
  await expect(page.getByRole('alert')).toContainText('Kunne ikke hente profilen')
  await layout(page, 'error')
  mode = 'ready'
  await page.getByRole('button', { name: 'Prøv igjen' }).click()
  await expect(page.getByText('Du er ikke med i noen turneringer ennå.')).toBeVisible()
  await expect(page.getByText(/Du har ingen spillerprofil ennå/)).toBeVisible()
  await layout(page, 'empty-unlinked')
  mode = 'inactive'
  await page.unroute('**/api/me/tournaments')
  await page.route('**/api/me/tournaments', route => route.fulfill({ json: memberships.map(item => ({ ...item, tournament: { ...item.tournament, status: 'archived' } })) }))
  await page.getByRole('button', { name: 'Oppdater profil og turneringer' }).click()
  await expect(page.getByLabel('Handicap', { exact: true })).toBeDisabled()
  await expect(page.getByText('Arkivert', { exact: true })).toBeVisible()
  await layout(page, 'inactive')
})

test('creator and invitation forms accept multibyte minimum and explain overflow', async ({ page, browser }) => {
  const failures: string[] = []
  const observe = (target: Page) => {
    target.on('pageerror', error => failures.push(error.message))
    target.on('response', response => {
      const path = new URL(response.url()).pathname
      if (response.status() >= 400 && !(path === '/api/auth/session' && response.status() === 401)) failures.push(`${path}: ${response.status()}`)
    })
    target.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) failures.push(message.text()) })
    target.on('requestfailed', request => {
      if (!new URL(request.url()).pathname.endsWith('/live')) failures.push(request.failure()?.errorText ?? 'Failed request')
    })
  }
  const checkLayout = async (target: Page, state: string) => {
    for (const width of [320, 390, 1280]) {
      await target.setViewportSize({ width, height: 900 })
      expect(await target.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
      await target.screenshot({ path: `/tmp/golf-profile-${state}-${width}.png`, fullPage: true })
    }
  }
  observe(page)
  await page.goto('/create')
  await page.getByLabel('Turneringsnavn', { exact: true }).fill(`Passordtest ${Date.now()}`)
  await page.getByRole('button', { name: 'Neste' }).click()
  await page.getByRole('heading', { name: 'Planlegg rundene' }).waitFor()
  await page.getByRole('button', { name: 'Neste' }).click()
  await page.getByLabel(/^Visningsnavn/).fill('Passordspiller')
  await page.getByLabel(/^Brukernavn/).fill(`utf8_${Date.now()}`)
  await page.getByLabel(/^Handicapindeks/).fill('14,4')
  await page.getByLabel(/^Passord/).fill('ø'.repeat(65))
  await page.getByRole('button', { name: 'Neste' }).click()
  await expect(page.getByText(/Passordet er for langt/)).toBeVisible()
  await checkLayout(page, 'creator-overflow')
  await page.getByLabel(/^Passord/).fill('ø'.repeat(6))
  await page.getByRole('button', { name: 'Neste' }).click()
  await expect(page.getByRole('heading', { name: 'Kontroller opplysningene' })).toBeVisible()
  const response = page.waitForResponse(r => new URL(r.url()).pathname === '/api/onboarding/tournaments' && r.request().method() === 'POST')
  await page.getByRole('button', { name: 'Opprett turnering', exact: true }).click()
  const created = await response
  expect(created.status()).toBe(201)
  const data = decodeObject(await created.json(), 'onboarding')
  const tournament = decodeTournament(data.tournament)
  const session = decodeAuthSession(data.session)
  await expect(page.getByLabel('Lenke til deltakerne')).toHaveValue(/#token=.+/)
  const inviteResponse = await page.request.post(`/api/tournaments/${tournament.id}/invitations`, {
    headers: { 'x-csrf-token': session.csrf_token },
    data: { expires_at: new Date(Date.now() + 86_400_000).toISOString(), max_uses: 1 },
  })
  expect(inviteResponse.status()).toBe(201)
  const invite = decodeObject(await inviteResponse.json(), 'invitation')
  const id = decodeUuid(invite.id, 'invitation.id')
  const token = decodeString(invite.token, 'invitation.token')
  const context = await browser.newContext()
  try {
    const guest = await context.newPage()
    observe(guest)
    await guest.goto(`/join/${id}#token=${token}`)
    const registration = guest.locator('section').filter({ has: guest.getByRole('heading', { name: 'Ny spiller', exact: true }) }).last()
    await registration.getByLabel(/^Visningsnavn/).fill('Invitert spiller')
    await registration.getByLabel(/^Brukernavn/).fill(`guest_${Date.now()}`)
    await registration.getByLabel(/^Handicapindeks/).fill('14,4')
    await registration.getByLabel(/^Passord/).fill('🏌'.repeat(33))
    await registration.getByRole('button', { name: 'Opprett konto og bli med' }).click()
    await expect(registration.getByRole('alert')).toContainText('Grensen er 128 byte')
    await checkLayout(guest, 'invitation-overflow')
    await registration.getByLabel(/^Passord/).fill('ø'.repeat(6))
    await registration.getByRole('button', { name: 'Opprett konto og bli med' }).click()
    await expect(registration).toBeHidden()
    await expect(guest.getByRole('heading', { name: 'Du er med!' })).toBeVisible()
    const joined = await guest.request.get('/api/auth/session')
    expect(joined.status()).toBe(200)
    expect(decodeAuthSession(await joined.json()).username).toMatch(/^guest_/)
    expect(failures).toEqual([])
  } finally { await context.close() }
})
