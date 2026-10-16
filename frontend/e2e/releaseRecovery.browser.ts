import { execFileSync } from 'node:child_process'
import { expect, test, type Page } from '@playwright/test'
import { decodeObject } from '../src/api/decoder'
import { decodeAuthSession } from '../src/api/auth'
import { decodeFourBallScoring } from '../src/api/fourBall/decoders'
import { at, claimManager, releaseFixture } from './fantasyReleaseSupport'
import { stablefordFixture } from './stablefordSupport'

test.skip(process.env.GOLF_RELEASE_RECOVERY_BROWSER !== '1', 'Requires the isolated release-recovery API/database and Chrome.')
function observe(page: Page, denied?: { round: string; tournament: string }, scoringRound?: string) {
  const errors: string[] = [], failures: string[] = [], responses: Promise<void>[] = []
  page.on('pageerror', e => errors.push(e.message))
  page.on('console', m => { if (m.type() === 'error' && !m.text().includes('Failed to load resource')) errors.push(m.text()) })
  page.on('response', r => {
    if (r.status() < 400) return
    responses.push((async () => {
      const path = new URL(r.url()).pathname, body: unknown = await r.json()
      const error = decodeObject(decodeObject(body, 'response').error, 'error')
      const allowed = r.status() === 401 && error.code === 'unauthenticated' && path === '/api/auth/session'
        || scoringRound && r.status() === 409 && error.code === 'round_not_editable' && path.startsWith(`/api/rounds/${scoringRound}/four-ball/scorecards/`) && path.endsWith('/scoring')
        || r.status() === 409 && error.code === 'fantasy_conflict' && path.includes('/fantasy/')
        || denied && r.status() === 503 && error.code === 'test_lost_ack' && path === `/api/rounds/${denied.round}/awards`
        || denied && r.status() === 403 && error.code === 'forbidden' && [
          `/api/rounds/${denied.round}/awards`, `/api/tournaments/${denied.tournament}/final-round-visibility`,
          `/api/tournaments/${denied.tournament}/result-share`,
        ].includes(path)
      if (!allowed) errors.push(`HTTP ${r.status()} ${path}: ${String(error.code)}`)
    })().catch((error: unknown) => { errors.push(error instanceof Error ? error.message : 'Response inspection failed') }))
  })
  page.on('requestfailed', r => { if (!r.url().includes('/live') && !r.failure()?.errorText.includes('ERR_ABORTED')) failures.push(new URL(r.url()).pathname) })
  return async () => { await Promise.all(responses); expect(errors).toEqual([]); expect(failures).toEqual([]) }
}
async function layout(page: Page, selector: string, name: string) {
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    const region = page.locator(selector); await region.scrollIntoViewIfNeeded()
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const button of await region.getByRole('button').all()) if (await button.isVisible()) expect((await button.boundingBox())?.height).toBeGreaterThanOrEqual(44)
    await region.screenshot({ path: `/tmp/golf-release-recovery-${name}-${width}.png` })
  }
}
for (const replay of [false, true]) test(`actual management parent exits after admin revocation ${replay ? 'on exact replay after committed original' : 'during initial award'}`, async ({ page }) => {
  const f = await stablefordFixture(page), verify = observe(page, { round: f.round.id, tournament: f.tournament.id })
  for (let hole = 1; hole <= 18; hole++) await f.save(hole, { type: 'numeric', gross_strokes: 4 })
  await f.mutate(`${f.cardPath}/confirm`); await f.mutate(`/api/rounds/${f.round.id}/complete`)
  const session = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
  const path = `/api/rounds/${f.round.id}/awards`, writes: unknown[] = []
  let release: () => void = () => undefined
  const held = new Promise<void>(resolve => { release = resolve })
  await page.route(`**${path}`, async route => {
    if (route.request().method() !== 'PUT') return route.continue()
    writes.push(route.request().postDataJSON())
    if (replay && writes.length === 1) {
      const response = await route.fetch(); expect(response.status()).toBe(200)
      return route.fulfill({ status: 503, json: { error: { code: 'test_lost_ack', message: 'Receipt deliberately unavailable' } } })
    }
    await held
    const response = await route.fetch(); expect(response.status()).toBe(403)
    return route.fulfill({ response })
  })
  await page.goto(`/manage/tournaments/${f.tournament.id}?round=${f.round.id}#lifecycle`)
  await layout(page, '.round-awards', replay ? 'empty-replay' : 'empty-initial')
  const group = page.getByRole('group', { name: 'Closest to pin · valgfri −1' })
  await group.getByLabel('Vinner av Closest to pin').selectOption(f.owner.id)
  await group.getByLabel('Begrunnelse for Closest to pin').fill('Bekreftet måling')
  await group.getByRole('button', { name: 'Lagre Closest to pin' }).click()
  if (replay) {
    const retry = page.getByRole('button', { name: 'Avklar samme Closest to pin-innsending' })
    await expect(retry).toBeEnabled(); await retry.click()
  }
  await expect.poll(() => writes.length).toBe(replay ? 2 : 1)
  await expect(page.getByRole('button', { name: 'Oppgi lokal gjenoppretting og forlat siden' })).toBeDisabled()
  await layout(page, '.award-recovery', replay ? 'pending-replay' : 'pending-initial')
  // Test-only local SQL removes exact tournament authority, retaining the live session.
  const revoked = execFileSync('podman', ['exec', '-i', 'golf-release-recovery-db', 'psql', '-U', 'golf', '-d', 'golf', '-v', `trip=${f.tournament.id}`, '-v', `actor=${session.user_id}`], {
    input: "UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=:'trip'::uuid AND user_id=:'actor'::uuid;", stdio: ['pipe', 'pipe', 'pipe'],
  })
  expect(revoked.toString()).toContain('UPDATE 1')
  await page.getByRole('button', { name: 'Oppdater oversikten', exact: true }).click()
  await expect(page.getByRole('heading', { name: 'Ingen tilgang' })).toBeVisible()
  release()
  await expect(page.getByText(/Utfallet av den opprinnelige innsendingen er fortsatt uavklart/)).toBeVisible()
  await expect(page.getByRole('button', { name: 'Logg ut' })).toBeDisabled()
  await layout(page, '.award-recovery', replay ? 'denied-replay' : 'denied-initial')
  const sessionAfter = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
  expect(sessionAfter.user_id).toBe(session.user_id); expect(sessionAfter.csrf_token).toBe(session.csrf_token)
  await page.getByRole('link', { name: 'Til turneringer', exact: true }).click()
  await expect(page).toHaveURL(new RegExp(`/manage/tournaments/${f.tournament.id}`))
  await page.getByRole('button', { name: 'Oppgi lokal gjenoppretting og forlat siden' }).click()
  await expect(page).toHaveURL('/tournaments'); await expect(page.getByRole('button', { name: 'Logg ut' })).toBeEnabled()
  expect(writes).toHaveLength(replay ? 2 : 1)
  if (replay) expect(writes[1]).toEqual(writes[0])
  const history = execFileSync('podman', ['exec', '-i', 'golf-release-recovery-db', 'psql', '-U', 'golf', '-d', 'golf', '-At', '-v', `round=${f.round.id}`], { input: "SELECT count(*) FROM round_award_history WHERE round_id=:'round'::uuid;", encoding: 'utf8' }).trim()
  expect(history).toBe(replay ? '1' : '0')
  await page.getByRole('button', { name: 'Logg ut' }).click(); await expect(page.getByRole('heading', { name: 'Logg inn', exact: true })).toBeVisible(); expect((await page.request.get('/api/auth/session')).status()).toBe(401); expect(writes).toHaveLength(replay ? 2 : 1)
  await verify()
})

test('Fantasy four-ball blank partner confirms sport, guards normal lock, then deliberate resolution requires reconfirmation', async ({ page, browser }) => {
  const f = await releaseFixture(page, ['four_ball_stroke_play'], [0, 0, 0, 0]), verify = observe(page, undefined, at(f.rounds, 0).id)
  const round = at(f.rounds, 0), teams = await f.configure(0, [0, 1, 2, 3], true), team = at(teams, 0)
  const scorer = await claimManager(f, browser, 1), verifyScorer = observe(scorer.page, undefined, round.id)
  try {
  await f.start(); await f.mutate(`/api/rounds/${round.id}/open`)
  const path = (id: string) => `/api/rounds/${round.id}/four-ball/scorecards/${id}`
  const save = async (player: string, hole: string, input: { type: 'numeric'; gross_strokes: number } | { type: 'no_score' }) => f.mutate(`/api/rounds/${round.id}/four-ball/inputs/conditional`, { request_id: crypto.randomUUID(), owner: { type: 'player', id: player }, hole_id: hole, input, expected_score: { type: 'absent' } }, 'PUT')
  for (const [index, side] of teams.entries()) {
    const card = decodeFourBallScoring(await (await page.request.get(`${path(side.id)}/scoring`)).json(), round.id, side.id)
    for (const hole of card.holes) for (const [partnerIndex, player] of hole.players.entries()) if (index !== 0 || partnerIndex === 0) await save(player.player_id, hole.hole_id, { type: 'numeric', gross_strokes: 4 })
    if (index !== 0) await f.mutate(`${path(side.id)}/confirm`)
  }
  const url = `/score?${new URLSearchParams({ tournament: f.tournament.id, round: round.id, owner_type: 'team', owner: team.id, view: 'summary' })}`
  await scorer.page.goto(url)
  await expect(scorer.page.getByText('Fantasy venter på partnerfelt.', { exact: false })).toBeVisible()
  const hidden = await (await scorer.page.request.get(`/api/rounds/${round.id}/completion-validation`)).json()
  expect(hidden).toMatchObject({ visibility: { mode: 'front_nine' }, issues: [] })
  await layout(scorer.page, '.scoring-notice', 'hidden-final-scorer')
  await page.goto(url)
  await expect(page.getByText('Fantasy venter på partnerfelt.', { exact: false })).toBeVisible()
  await expect(page.getByText(/hull 1, 2, 3, 4/)).toBeVisible(); await layout(page, '.four-ball-confirm', 'sport-confirm')
  await layout(page, '.scoring-notice', 'fantasy-warning')
  await page.getByRole('checkbox', { name: 'Jeg bekrefter at tomme partnerfelt ikke bidrar med score.' }).check()
  await page.getByRole('button', { name: 'Bekreft lagets scorekort', exact: true }).click()
  await expect(page.getByText('Lagets scorekort er bekreftet.', { exact: true })).toBeVisible()
  expect((await f.board(0)).round.points.state).toBe('pending')
  await f.mutate(`/api/rounds/${round.id}/complete`)
  const manage = `/manage/tournaments/${f.tournament.id}?round=${round.id}#lifecycle`
  await page.goto(manage); await expect(page.getByRole('button', { name: 'Lås runden', exact: true })).toBeDisabled()
  await expect(page.getByRole('link', { name: /Avklar partnerfelt og bekreft på nytt/ })).toBeVisible()
  await layout(page, '.round-lifecycle-detail', 'guarded-lock')
  const guarded = await page.request.post(`/api/rounds/${round.id}/lock`, { headers: { 'x-csrf-token': f.auth.csrf_token } })
  expect(guarded.status()).toBe(409); expect(await guarded.json()).toMatchObject({ error: { code: 'round_fantasy_inputs_unresolved' } })
  await page.getByRole('link', { name: /Avklar partnerfelt og bekreft på nytt/ }).click()
  await expect(page.getByRole('button', { name: 'Korriger score' })).toBeVisible()
  const card = decodeFourBallScoring(await (await page.request.get(`${path(team.id)}/scoring`)).json(), round.id, team.id)
  for (const hole of card.holes) for (const player of hole.players) if (player.score === null) await save(player.player_id, hole.hole_id, { type: 'no_score' })
  await page.goto(manage); await expect(page.getByRole('button', { name: 'Lås runden', exact: true })).toBeDisabled()
  await expect(page.getByText(/Mangler bekreftelse/).first()).toBeVisible()
  await page.goto(url); await page.getByRole('button', { name: 'Bekreft lagets scorekort', exact: true }).click()
  await expect(page.getByText('Lagets scorekort er bekreftet.', { exact: true })).toBeVisible()
  await page.goto(manage); await expect(page.getByRole('button', { name: 'Lås runden', exact: true })).toBeEnabled()
  await page.getByRole('button', { name: 'Lås runden', exact: true }).click(); await page.getByRole('button', { name: 'Bekreft og lås runden' }).click()
  await expect(page.getByText('Runden er låst. Scorekortene er skrivebeskyttet.')).toBeVisible()
  expect((await f.board(0)).round.points.state).toBe('settled')
  await layout(page, '.round-lifecycle-detail', 'resolved-locked'); await verify(); await verifyScorer()
  } finally { await scorer.context.close() }
})
