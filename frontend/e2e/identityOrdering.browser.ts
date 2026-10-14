import { test, expect } from '@playwright/test'
import { decodeObject, decodeUuid } from '../src/api/decoder'
import { decodeAuthSession } from '../src/api/auth'
import { currentIsAnders, deferred, enterInvitation, enterLogin, fillRegistration, invitation, layout, observe, reviewCreation, unique } from './identitySupport'

test.skip(process.env.GOLF_IDENTITY_BROWSER !== '1', 'Requires disposable seeded API and GOLF_IDENTITY_BROWSER=1.')
test.use({ screenshot: 'off', trace: 'off' })

for (const width of [390, 1280]) {
  for (const flow of ['onboarding', 'invitation'] as const) {
    test(`${flow} response cannot overtake a newer login at ${width}px`, async ({ page, browser }) => {
      await page.setViewportSize({ width, height: 900 })
      const events = observe(page)
      let loginRequests = 0
      page.on('request', request => { if (new URL(request.url()).pathname === '/api/auth/login') loginRequests++ })
      const committed = deferred<string>()
      const release = deferred<void>()
      let writes = 0
      const path = flow === 'onboarding' ? '**/api/onboarding/tournaments' : '**/api/invitations/*/register'
      await page.route(path, async route => {
        writes++
        const response = await route.fetch()
        expect(response.status()).toBe(201)
        const body = decodeObject(await response.json(), 'response')
        committed.resolve(flow === 'onboarding'
          ? decodeUuid(decodeObject(body.tournament, 'tournament').id, 'tournament.id')
          : decodeUuid(body.tournament_id, 'tournament_id'))
        await release.promise
        await route.fulfill({ response })
      })
      try {
        if (flow === 'onboarding') {
          await reviewCreation(page, unique())
          await page.getByRole('button', { name: 'Opprett turnering', exact: true }).click()
        } else {
          await enterInvitation(page, await invitation(browser))
          await fillRegistration(page)
        }
        const departedTournament = await committed.promise
        await enterLogin(page)
        await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
        await expect(page.getByRole('button', { name: 'Logger inn...' })).toBeDisabled()
        await layout(page, width)
        expect(loginRequests).toBe(0)
        release.resolve()
        await currentIsAnders(page, departedTournament)
        expect(loginRequests).toBe(1)
        expect(writes).toBe(1)
        await expect(page.getByRole('heading', { name: 'Du er med!' })).toHaveCount(0)
        await expect(page.getByLabel('Lenke til deltakerne')).toHaveCount(0)
        await layout(page, width)
        expect(events.errors).toEqual([])
        expect(events.failures).toEqual([])
      } finally { release.resolve() }
    })
  }
}

test('ordinary onboarding retains the accepted one-time receipt across reconciliation', async ({ page }) => {
  const events = observe(page)
  const username = unique()
  await reviewCreation(page, username)
  await page.getByRole('button', { name: 'Opprett turnering', exact: true }).click()
  await expect(page.getByLabel('Lenke til deltakerne')).toBeVisible()
  const actual = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
  expect(actual.username).toBe(username)
  for (const width of [320, 390, 1280]) {
    await layout(page, width)
    await page.screenshot({ path: `/tmp/golf-a3-onboarding-${width}.png`, fullPage: true, mask: [page.getByLabel('Lenke til deltakerne')] })
  }
  expect(events.errors).toEqual([])
  expect(events.failures).toEqual([])
})

test('invitation loading, error retry, long names and accepted registration remain usable', async ({ page, browser }) => {
  const events = observe(page)
  const created = await invitation(browser)
  let fail = true
  await page.route('**/api/invitations/*/preview', route => fail
    ? route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Midlertidig feil' } } })
    : route.continue())
  await page.goto(`/join/${created.invitation.id}#token=${created.invitation.token}`)
  await expect(page.getByRole('button', { name: 'Prøv igjen' })).toBeVisible()
  await layout(page, 320)
  fail = false
  await page.getByRole('button', { name: 'Prøv igjen' }).click()
  await expect(page.getByRole('heading', { name: 'Ny spiller' })).toBeVisible()
  for (const width of [320, 390, 1280]) await layout(page, width)
  await fillRegistration(page)
  await expect(page.getByRole('heading', { name: 'Du er med!' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'Åpne turneringen' })).toHaveAttribute('href', `/tournaments/${created.tournament.id}`)
  await layout(page, 390)
  await page.screenshot({ path: '/tmp/golf-a3-invitation-success.png', fullPage: true })
  expect(events.errors).toEqual([])
  expect(events.failures).toEqual([])
})

test('a late anonymous session read cannot replace a newer login', async ({ page }) => {
  const seen = deferred<void>()
  const release = deferred<void>()
  let first = true
  await page.route('**/api/auth/session', async route => {
    if (!first) return route.continue()
    first = false
    const response = await route.fetch()
    expect(response.status()).toBe(401)
    seen.resolve()
    await release.promise
    await route.fulfill({ response })
  })
  try {
    await page.goto('/login?returnTo=%2Ftournaments')
    await seen.promise
    await page.getByLabel('Brukernavn', { exact: true }).fill('anders')
    await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
    await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
    await currentIsAnders(page)
    release.resolve()
    await expect(page.locator('.session-bar')).toContainText('Anders')
    await currentIsAnders(page)
  } finally { release.resolve() }
})
