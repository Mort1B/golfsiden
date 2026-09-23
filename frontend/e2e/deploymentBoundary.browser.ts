import { expect, test } from '@playwright/test'
import { offlineFixture } from './offlineSupport'

const origin = process.env.GOLF_DEPLOYMENT_ORIGIN ?? 'https://127.0.0.1:54443'
const localCertificate = process.env.GOLF_DEPLOYMENT_LOCAL_CERT === '1'
test.skip(process.env.GOLF_DEPLOYMENT_BROWSER !== '1', 'Requires an isolated HTTPS production-mode API behind Caddy.')
test.use({ baseURL: origin, ignoreHTTPSErrors: localCertificate, screenshot: 'off', trace: 'off' })

test('HTTPS proxy preserves secure sessions, score persistence and native live results', async ({ page, browser }) => {
  const fixture = await offlineFixture(page)
  await fixture.save(2, 5)
  const cookies = await page.context().cookies()
  const session = cookies.find(cookie => cookie.httpOnly)
  expect(session?.secure).toBe(true)
  expect(session?.sameSite).toBe('Lax')
  const other = await browser.newContext({ baseURL: origin, ignoreHTTPSErrors: localCertificate, viewport: { width: 390, height: 844 } })
  try {
    const reader = await other.newPage()
    await reader.addInitScript(() => {
      const Native = EventSource
      window.EventSource = class extends Native {
        constructor(url: string | URL, options?: EventSourceInit) {
          super(url, options)
          this.addEventListener('open', () => { document.documentElement.dataset.auditLiveOpen = '1' })
          this.addEventListener('score', () => {
            document.documentElement.dataset.auditScoreEvents = String(Number(document.documentElement.dataset.auditScoreEvents ?? '0') + 1)
          })
        }
      }
    })
    const errors: string[] = []
    const failedResponses: string[] = []
    for (const target of [page, reader]) {
      target.on('pageerror', error => errors.push(error.message))
      target.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text()) })
      target.on('response', response => {
        const path = new URL(response.url()).pathname
        if (response.status() >= 400 && !(response.status() === 401 && path === '/api/auth/session')) failedResponses.push(`${response.status()} ${path}`)
      })
      target.on('requestfailed', request => {
        if (request.failure()?.errorText !== 'net::ERR_ABORTED') errors.push(`${new URL(request.url()).pathname}: ${request.failure()?.errorText}`)
      })
    }
    const liveOrigins: string[] = []
    reader.on('request', request => { if (new URL(request.url()).pathname.endsWith('/live')) liveOrigins.push(new URL(request.url()).origin) })
    const response = await page.goto(fixture.url())
    expect(response?.headers()['content-security-policy']).toContain("script-src 'self'")
    expect(response?.headers()['strict-transport-security']).toContain('max-age=')
    expect(response?.headers()['x-content-type-options']).toBe('nosniff')
    await reader.goto('/login')
    await reader.getByLabel('Brukernavn', { exact: true }).fill(fixture.username)
    await reader.getByLabel('Passord', { exact: true }).fill(fixture.password)
    await reader.getByRole('button', { name: 'Logg inn', exact: true }).click()
    await expect(reader).not.toHaveURL(/\/login/)
    await reader.goto(`/leaderboard?tournament=${fixture.tournament.id}&round=${fixture.round.id}&scope=round&metric=gross`)
    await expect(reader.locator('.leaderboard-row-link')).toHaveCount(1)
    await expect.poll(() => liveOrigins.length).toBeGreaterThan(0)
    // The stream's first keepalive can precede the native open notification.
    await expect.poll(() => reader.evaluate(() => document.documentElement.dataset.auditLiveOpen), { timeout: 30_000 }).toBe('1')
    await expect(reader.locator('.leaderboard-score span')).toHaveText('5 brutto')
    const scoreEvents = await reader.evaluate(() => Number(document.documentElement.dataset.auditScoreEvents ?? '0'))
    await expect(page.getByRole('button', { name: /Registrer par/ })).toBeEnabled()
    await page.getByRole('button', { name: /Registrer par/ }).click()
    await expect(page.getByText('Lagret på serveren', { exact: true })).toBeVisible()
    await expect.poll(async () => (await fixture.read()).holes[0]?.score?.gross_strokes).toBe(4)
    await expect.poll(() => reader.evaluate(() => Number(document.documentElement.dataset.auditScoreEvents ?? '0'))).toBeGreaterThan(scoreEvents)
    await expect(reader.locator('.leaderboard-score span')).toHaveText('9 brutto')
    await expect(reader.locator('.leaderboard-progress')).toContainText('2 av 18 hull')
    expect(liveOrigins.every(value => value === origin)).toBe(true)
    await reader.reload()
    await expect(reader.locator('.leaderboard-score span')).toHaveText('9 brutto')
    expect(errors).toEqual([])
    expect(failedResponses).toEqual([])
  } finally { await other.close() }
})
