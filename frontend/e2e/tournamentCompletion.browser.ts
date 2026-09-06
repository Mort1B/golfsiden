import { test, expect, type Page, type APIRequestContext } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeObject } from '../src/api/decoder'
import { decodeTournamentList, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeCompletionValidation, decodeScoringScorecard, ownerTypeForFormat } from '../src/api/scorecards'

test.skip(process.env.GOLF_TOURNAMENT_COMPLETION_BROWSER !== '1', 'Requires fresh disposable seed and GOLF_TOURNAMENT_COMPLETION_BROWSER=1.')

async function login(page: Page, username: string) {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill(username)
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
}
async function mutate(api: APIRequestContext, path: string, csrf: string, data: unknown = {}, method = 'POST') {
  const response = await api.fetch(path, { method, data, headers: { 'x-csrf-token': csrf } })
  expect(response.ok(), `${method} ${path}: ${response.status()}`).toBe(true)
}
async function layout(page: Page, state: string) {
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    const panel = page.locator('.tournament-completion')
    await panel.scrollIntoViewIfNeeded()
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const button of await panel.getByRole('button').all()) {
      expect((await button.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await button.isEnabled()) await button.click({ trial: true })
    }
    await panel.screenshot({ path: `/tmp/golf-tournament-completion-${state}-${width}.png` })
  }
}

test('completion readiness, confirmation, concurrent completion and preserved member visibility', async ({ page, browser }) => {
  await login(page, 'admin')
  const errors: string[] = []
  const failed: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
  page.on('response', (response) => { if (response.status() >= 400) failed.push(`${response.status()} ${new URL(response.url()).pathname}`) })
  page.on('requestfailed', (request) => { if (!request.url().includes('/live') && !request.failure()?.errorText.includes('ERR_ABORTED')) failed.push(new URL(request.url()).pathname) })
  const api = page.request
  const auth = decodeAuthSession(await (await api.get('/api/auth/session')).json())
  const trip = decodeTournamentList(await (await api.get('/api/tournaments')).json()).find((item) => item.name === 'Guttas Golf 2026')
  if (!trip || trip.status !== 'draft') throw new Error('Fresh seed required')
  const rounds = decodeTournamentRounds(await (await api.get(`/api/tournaments/${trip.id}/rounds`)).json(), trip.id)
  const url = `/manage/tournaments/${trip.id}#lifecycle`
  const panel = page.locator('.tournament-completion')
  const action = panel.getByRole('button', { name: 'Fullfør turneringen', exact: true })
  await page.goto(url)
  await expect(action).toBeDisabled()
  await expect(panel.getByText(/Start turneringen/)).toBeVisible()
  await layout(page, 'draft')
  await mutate(api, `/api/tournaments/${trip.id}/start`, auth.csrf_token, { expected_tournament_updated_at: trip.updated_at })
  for (const round of rounds) {
    await mutate(api, `/api/rounds/${round.id}/open`, auth.csrf_token)
    const progress = decodeCompletionValidation(await (await api.get(`/api/rounds/${round.id}/completion-validation`)).json(), round.id, ownerTypeForFormat(round.scoring_format))
    for (const { owner } of progress.owners) {
      const path = `/api/rounds/${round.id}/scorecards/${owner.type}/${owner.id}`
      const card = decodeScoringScorecard(await (await api.get(`${path}/scoring`)).json(), round.id, owner)
      for (const hole of card.holes) await mutate(api, `/api/rounds/${round.id}/scores`, auth.csrf_token, { owner, hole_id: hole.hole_id, gross_strokes: hole.par }, 'PUT')
      await mutate(api, `${path}/confirm`, auth.csrf_token)
    }
    await mutate(api, `/api/rounds/${round.id}/complete`, auth.csrf_token)
    if (round.round_number !== trip.number_of_rounds) await mutate(api, `/api/rounds/${round.id}/lock`, auth.csrf_token)
  }
  const final = rounds.find((round) => round.round_number === trip.number_of_rounds)
  if (!final) throw new Error('Final missing')
  await expect(panel.getByRole('link')).toHaveCount(1)
  await expect(panel.getByRole('link')).toHaveAttribute('href', `/manage/tournaments/${trip.id}?round=${final.id}#lifecycle`)
  await expect(action).toBeDisabled()
  await layout(page, 'blocked')
  const second = await browser.newContext()
  const secondPage = await second.newPage()
  await login(secondPage, 'admin')
  const secondAuth = decodeAuthSession(await (await second.request.get('/api/auth/session')).json())
  await mutate(second.request, `/api/rounds/${final.id}/lock`, secondAuth.csrf_token)
  await expect(action).toBeEnabled()
  await action.click()
  await expect(panel.getByRole('button', { name: 'Avbryt' })).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(action).toBeFocused()
  await action.click()
  await layout(page, 'confirmation')
  await secondPage.goto(url)
  const secondPanel = secondPage.locator('.tournament-completion')
  await secondPanel.getByRole('button', { name: 'Fullfør turneringen', exact: true }).click()
  await secondPanel.getByRole('button', { name: 'Bekreft og fullfør turneringen' }).click()
  await expect(secondPanel.getByRole('heading', { name: 'Turneringen er avsluttet' })).toBeVisible()
  await expect(panel.getByRole('heading', { name: 'Turneringen er avsluttet' })).toBeVisible()
  await expect(panel.getByRole('group')).toHaveCount(0)
  await expect(action).toHaveCount(0)
  await layout(page, 'completed')
  const visibility = decodeObject(await (await api.get(`/api/tournaments/${trip.id}/final-round-visibility`)).json(), 'visibility')
  expect(visibility.back_nine_hidden).toBe(true)
  const member = await browser.newContext()
  const memberPage = await member.newPage()
  await login(memberPage, 'anders')
  expect((await member.request.get(`/api/tournaments/${trip.id}`)).ok()).toBe(true)
  await memberPage.goto(`/rounds/${final.id}`)
  await expect(memberPage.getByText('Kun hull 1–9 vises. Fullføring og bekreftelse er skjult.')).toBeVisible()
  await memberPage.goto(url)
  await expect(memberPage.getByRole('heading', { name: 'Ingen tilgang' })).toBeVisible()
  await expect(memberPage.locator('.tournament-completion')).toHaveCount(0)
  expect(errors).toEqual([])
  expect(failed).toEqual([])

  // Deliberate edge-state injections follow the real authenticated workflow.
  await page.route(`**/api/tournaments/${trip.id}`, (route) => route.fulfill({ json: { ...trip, status: 'active', name: 'LangtTurneringsnavnUtenMellomrom'.repeat(8) } }))
  let mode: 'pending' | 'empty' | 'error' | 'long' = 'pending'
  let release: (() => void) | undefined
  await page.route(`**/api/tournaments/${trip.id}/rounds`, async (route) => {
    if (mode === 'pending') await new Promise<void>((resolve) => { release = resolve })
    if (mode === 'error') { await route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Unavailable' } } }); return }
    await route.fulfill({ json: mode === 'long' ? rounds.map((round) => ({ ...round, status: 'locked' })) : [] })
  })
  await page.reload()
  await expect.poll(() => Boolean(release)).toBe(true)
  await expect(panel.getByRole('status')).toBeVisible()
  await expect(action).toBeDisabled()
  await layout(page, 'loading')
  mode = 'empty'
  release?.()
  await expect(panel.getByText(/Rundeplanen er ufullstendig/)).toBeVisible()
  await layout(page, 'empty')
  mode = 'error'
  await page.reload()
  await expect(panel.getByText('Rundeplanen kunne ikke kontrolleres. Prøv å oppdatere kontrollen.')).toBeVisible({ timeout: 20000 })
  await expect(action).toBeDisabled()
  await layout(page, 'error')
  mode = 'long'
  await panel.getByRole('button', { name: 'Oppdater fullføringskontrollen' }).click()
  await expect(action).toBeEnabled()
  await action.click()
  await layout(page, 'long')
  await second.close()
  await member.close()
})
