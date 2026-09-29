// Run from the repository root against dedicated, seeded local services only.
// `before` creates synthetic data; `after` verifies it after an operator restart.
import { chromium, expect } from '../../../frontend/node_modules/@playwright/test/index.mjs'
import { mkdir, readFile, writeFile } from 'node:fs/promises'

const origin = 'http://127.0.0.1:5173'
const artifacts = '/tmp/golf-test-ready-20260929'
const phase = process.argv[2]
if (!['before', 'after'].includes(phase)) throw new Error('Use before or after')
await mkdir(artifacts, { recursive: true })
const browser = await chromium.launch({ channel: 'chrome', headless: true })
const errors = []
const checks = []
const context = await browser.newContext({ baseURL: origin, viewport: { width: 390, height: 844 } })
const page = await context.newPage()
function observe(target) {
  target.on('pageerror', error => errors.push(error.message))
  target.on('console', message => {
    if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text())
  })
  target.on('response', response => {
    const path = new URL(response.url()).pathname
    if (response.status() >= 400 && !(response.status() === 401 && path === '/api/auth/session')) {
      errors.push(`${response.status()} ${path}`)
    }
  })
  target.on('requestfailed', request => {
    if (!request.url().includes('/live') && !request.failure()?.errorText.includes('ERR_ABORTED')) {
      errors.push(`${new URL(request.url()).pathname}: ${request.failure()?.errorText}`)
    }
  })
}
observe(page)
async function read(target, path) {
  const response = await target.request.get(path)
  expect(response.ok(), `${path}: ${response.status()}`).toBe(true)
  return response.json()
}
async function login(target, username, password) {
  await target.goto('/login')
  await target.getByLabel('Brukernavn', { exact: true }).fill(username)
  await target.getByLabel('Passord', { exact: true }).fill(password)
  await target.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(target).not.toHaveURL(/\/login/)
}
async function layout(target, state) {
  for (const [width, height] of [[320, 600], [390, 844], [1280, 900]]) {
    await target.setViewportSize({ width, height })
    expect(await target.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    await target.screenshot({ path: `${artifacts}/${phase}-${state}-${width}.png`, fullPage: true })
  }
}
const cardPath = item => `/api/rounds/${item.roundId}/scorecards/${item.owner.type}/${item.owner.id}/scoring`
const scoreUrl = (tournamentId, item) => `/score?${new URLSearchParams({
  tournament: tournamentId, round: item.roundId, owner_type: item.owner.type,
  owner: item.owner.id, hole: '1', view: 'summary',
})}`
try {
  if (phase === 'before') {
    const stamp = Date.now()
    const username = `ready_${stamp}`
    const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
    // Account/two-round fixture only; courses, team/flight assignment, opening
    // and the score mutations below use the actual browser UI.
    const response = await page.request.post('/api/onboarding/tournaments', { data: {
      creator: { account: { username, password: 'readiness-local-password' },
        player: { display_name: 'Arrangør for praktisk funksjonstest', handicap_index: 10 } },
      tournament: { name: `Praktisk funksjonstest ${stamp}`, description: '', start_date: day,
        end_date: day, counted_rounds: 1, mandatory_round_number: null },
      rounds: ['team_scramble', 'individual_stroke_play'].map((scoring_format, index) => ({
        round_number: index + 1, name: index === 0 ? 'Lagets runde' : 'Individuell runde',
        round_date: day, scoring_format,
      })),
    } })
    expect(response.status()).toBe(201)
    const { tournament } = await response.json()
    const rounds = await read(page, `/api/tournaments/${tournament.id}/rounds`)
    await page.goto(`/tournaments/${tournament.id}/invitations`)
    await page.getByLabel('Utløper', { exact: true }).fill(`${day}T22:00`)
    await page.getByLabel('Maks antall påmeldinger').fill('2')
    await page.getByRole('button', { name: 'Opprett lenke', exact: true }).click()
    const invitation = await page.getByLabel('Del denne lenken').inputValue()
    const memberContext = await browser.newContext({ baseURL: origin, viewport: { width: 390, height: 844 } })
    const member = await memberContext.newPage()
    observe(member)
    await member.goto(invitation)
    const existing = member.locator('.existing-account')
    await existing.getByLabel('Brukernavn', { exact: true }).fill('anders')
    await existing.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
    await existing.getByRole('button', { name: 'Logg inn', exact: true }).click()
    await member.getByRole('button', { name: 'Bli med i turneringen', exact: true }).click()
    await expect(member.getByRole('heading', { name: 'Du er med!', exact: true })).toBeVisible()
    checks.push('Organizer invitation and existing-player joining through UI')
    for (const round of rounds) {
      await page.goto(`/manage/tournaments/${tournament.id}?round=${round.id}#courses`)
      const course = page.locator('.round-course-card').filter({ hasText: round.name })
      const configure = course.getByRole('button', { name: 'Konfigurer', exact: true })
      if (await configure.getAttribute('aria-expanded') !== 'true') await configure.click()
      await course.getByRole('combobox', { name: 'Lagret bane', exact: true }).selectOption({ label: 'Hacienda del Alamo Golf Club' })
      await course.getByRole('button', { name: 'Bruk lagret bane på runden', exact: true }).click()
      await expect(course.locator('.course-receipt')).toContainText('er lagret med Hacienda')
      const pairing = page.locator('.round-pairing-card').filter({ hasText: round.name })
      await pairing.getByRole('button', { name: 'Rediger', exact: true }).click()
      if (round.scoring_format === 'team_scramble') await pairing.getByRole('button', { name: 'Legg til lag', exact: true }).click()
      await pairing.getByRole('button', { name: 'Legg til flight', exact: true }).click()
      for (const assignment of await pairing.locator('.pairing-kind .pairing-assignments select').all()) {
        await assignment.selectOption({ index: 1 })
      }
      await pairing.getByRole('button', { name: 'Lagre hele oppsettet', exact: true }).click()
      await expect(pairing.locator('.pairing-save-row')).toContainText('Synkronisert med serveren')
      const pairings = await read(page, `/api/rounds/${round.id}/pairings`)
      expect(pairings.flights[0].members).toHaveLength(2)
      expect(pairings.teams).toHaveLength(round.scoring_format === 'team_scramble' ? 1 : 0)
      if (pairings.teams[0]) expect(pairings.teams[0].members).toHaveLength(2)
      await layout(page, `setup-${round.scoring_format}`)
    }
    await page.getByRole('button', { name: 'Start turneringen', exact: true }).click()
    const cards = []
    for (const round of rounds) {
      await page.goto(`/manage/tournaments/${tournament.id}?round=${round.id}#lifecycle`)
      await page.getByRole('button', { name: 'Åpne runden', exact: true }).click()
      await page.getByRole('button', { name: 'Bekreft og åpne runden', exact: true }).click()
      await expect(page.getByText('Runden er åpnet.', { exact: true })).toBeVisible()
      const access = await read(member, `/api/rounds/${round.id}/score-access`)
      const owner = access.writable_owners[0]
      expect(owner).toBeDefined()
      const item = { roundId: round.id, owner, format: round.scoring_format }
      await member.goto(scoreUrl(tournament.id, item).replace('view=summary', 'view=hole'))
      await member.getByRole('button', { name: /^Registrer par/ }).click()
      await expect(member.locator('.score-sync')).toHaveText('Lagret på serveren')
      const initial = await read(member, cardPath(item))
      await member.getByRole('button', { name: 'Legg til ett slag', exact: true }).click()
      await expect.poll(async () => (await read(member, cardPath(item))).holes[0].score?.gross_strokes)
        .toBe(initial.holes[0].par + 1)
      await expect(member.locator('.score-sync')).toHaveText('Lagret på serveren')
      const card = await read(member, cardPath(item))
      expect(card.holes[0].score.gross_strokes).toBe(card.holes[0].par + 1)
      expect(card.holes_scored).toBe(1)
      await member.goto(scoreUrl(tournament.id, item))
      await expect(member.locator('.scorecard-holes li').first()).toContainText(String(card.holes[0].par + 1))
      await layout(member, `score-${round.scoring_format}`)
      cards.push({ ...item, card })
    }
    checks.push('Saved courses, manual team/flight assignment, start/open, player score entry and editing for team and individual rounds')
    await member.reload()
    await member.getByRole('button', { name: 'Logg ut', exact: true }).click()
    await login(member, 'anders', 'golf-dev-2026')
    for (const item of cards) expect(await read(member, cardPath(item))).toEqual(item.card)
    checks.push('Exact team and individual scorecards retained after reload and sign-out/sign-in')
    const boards = {}
    for (const metric of ['gross', 'net']) {
      boards[metric] = await read(member, `/api/tournaments/${tournament.id}/leaderboards/${metric}`)
      await member.goto(`/leaderboard?tournament=${tournament.id}&scope=tournament&metric=${metric}`)
      await expect(member.getByRole('heading', { name: metric === 'gross' ? 'Brutto resultat' : 'Netto resultat', exact: true })).toBeVisible()
      await expect(member.locator('.leaderboard-row-link')).toHaveCount(2)
      await layout(member, metric)
    }
    await writeFile(`${artifacts}/persistence.json`, JSON.stringify({ tournamentId: tournament.id, cards, boards }), { mode: 0o600 })
    await memberContext.close()
  } else {
    const saved = JSON.parse(await readFile(`${artifacts}/persistence.json`, 'utf8'))
    await login(page, 'anders', 'golf-dev-2026')
    for (const item of saved.cards) {
      expect(await read(page, cardPath(item))).toEqual(item.card)
      await page.goto(scoreUrl(saved.tournamentId, item))
      await expect(page.locator('.scorecard-holes li').first()).toContainText(String(item.card.holes[0].score.gross_strokes))
      await layout(page, `score-${item.format}`)
    }
    for (const metric of ['gross', 'net']) {
      expect(await read(page, `/api/tournaments/${saved.tournamentId}/leaderboards/${metric}`)).toEqual(saved.boards[metric])
      await page.goto(`/leaderboard?tournament=${saved.tournamentId}&scope=tournament&metric=${metric}`)
      await expect(page.locator('.leaderboard-row-link')).toHaveCount(2)
      await layout(page, metric)
    }
    checks.push('Exact scorecards and gross/net results retained after API, frontend and PostgreSQL restart, in a fresh browser session')
  }
  expect(errors).toEqual([])
  checks.push('No unexpected console, page, HTTP or request failures; no horizontal overflow at 320, 390 and 1280px')
  await writeFile(`${artifacts}/${phase}-checks.json`, JSON.stringify({ checks, errors }, null, 2))
  console.log(JSON.stringify({ phase, checks, errors }, null, 2))
} finally {
  await browser.close()
}
