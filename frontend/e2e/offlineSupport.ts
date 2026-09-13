import { expect, type Page } from '@playwright/test'
import { decodeObject } from '../src/api/decoder'
import { decodeAuthSession } from '../src/api/auth'
import { decodeRound, decodeTournament, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeScoringScorecard } from '../src/api/scorecards'

export async function offlineFixture(page: Page) {
  const stamp = `${Date.now()}_${Math.floor(Math.random() * 10000)}`
  const username = `offline_${stamp}`; const password = 'offline-browser-password'
  const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
  const response = await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username, password }, player: { display_name: 'Spiller med et svært langt navn som fører score uten forbindelse', handicap_index: 12 } },
    tournament: { name: `Golfturnering ${stamp}`, description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
    rounds: [{ round_number: 1, name: 'Runden med et langt navn', round_date: day, scoring_format: 'individual_stroke_play' }],
  } })
  expect(response.status()).toBe(201)
  const body = decodeObject(await response.json(), 'onboarding')
  const tournament = decodeTournament(body.tournament); const auth = decodeAuthSession(body.session)
  const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${tournament.id}/rounds`)).json(), tournament.id)
  const first = rounds[0]; if (!first || !auth.player_id) throw new Error('Missing fixture round/player')
  async function mutate(path: string, data: unknown = {}, method = 'POST') {
    const result = await page.request.fetch(path, { method, data, headers: { 'x-csrf-token': auth.csrf_token } })
    expect(result.ok(), `Fixture mutation status ${result.status()}`).toBe(true)
    return result
  }
  const round = decodeRound(await (await mutate(`/api/rounds/${first.id}/course-configuration`, {
    expected_round_updated_at: first.updated_at, selection: { source: 'manual', course_name: 'Testbane', location: null,
      tee: { category: 'male', name: 'Gul', course_rating: 72, slope_rating: 113, holes: Array.from({ length: 18 }, (_, index) => ({ par: 4, stroke_index: index + 1, distance: null })) } },
  }, 'PUT')).json())
  await mutate(`/api/rounds/${round.id}/pairings`, { expected_round_updated_at: round.updated_at, teams: [], flights: [{ id: crypto.randomUUID(), name: 'Flight 1', starting_hole: 1, tee_time: null, members: [{ player_id: auth.player_id }] }], legacy_conversions: [] }, 'PUT')
  const fresh = decodeTournament(await (await page.request.get(`/api/tournaments/${tournament.id}`)).json())
  await mutate(`/api/tournaments/${tournament.id}/start`, { expected_tournament_updated_at: fresh.updated_at })
  await mutate(`/api/rounds/${round.id}/open`)
  const owner = { type: 'player' as const, id: auth.player_id }
  const read = async () => decodeScoringScorecard(await (await page.request.get(`/api/rounds/${round.id}/scorecards/player/${owner.id}/scoring`)).json(), round.id, owner)
  const card = await read()
  const save = async (number: number, gross_strokes: number) => {
    const hole = card.holes[number - 1]; if (!hole) throw new Error('Missing fixture hole')
    await mutate(`/api/rounds/${round.id}/scores`, { owner, hole_id: hole.hole_id, gross_strokes }, 'PUT')
  }
  const url = (hole = 1, view = 'hole') => `/score?${new URLSearchParams({ tournament: tournament.id, round: round.id, owner_type: owner.type, owner: owner.id, hole: String(hole), view })}`
  return { auth, owner, card, round, tournament, username, password, read, save, mutate, url }
}
export async function queueRows(page: Page): Promise<unknown[]> {
  return page.evaluate(async () => new Promise<unknown[]>((resolve, reject) => {
    const request = indexedDB.open('golf-pending-scores-v1', 1)
    request.onerror = () => reject(new Error('Queue unavailable'))
    request.onsuccess = () => {
      const db = request.result
      const tx = db.transaction('pending'); const read = tx.objectStore('pending').getAll()
      read.onsuccess = () => { const result: unknown[] = read.result; resolve(result) }
      read.onerror = () => reject(new Error('Queue unreadable'))
      tx.oncomplete = () => db.close()
    }
  }))
}
export async function offlineLayout(page: Page, name: string, selector = '.main-content') {
  for (const [width, height] of [[320, 600], [390, 844], [1280, 900]] as const) {
    await page.setViewportSize({ width, height })
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    for (const control of await page.locator(`${selector} button:visible, ${selector} summary:visible`).all()) {
      expect((await control.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await control.isEnabled()) { await control.scrollIntoViewIfNeeded(); await control.click({ trial: true }) }
    }
    for (const row of await page.locator('.stableford-result > div').all()) {
      const label = await row.locator('dt').boundingBox(), value = await row.locator('dd').boundingBox()
      expect(label).not.toBeNull(); expect(value).not.toBeNull()
      if (label && value) expect(label.x + label.width).toBeLessThanOrEqual(value.x)
    }
    await page.evaluate(() => window.scrollTo(0, 0))
    await page.screenshot({ path: `/tmp/golf-offline-${name}-${width}.png`, fullPage: true })
  }
}
export function offlineEvents(page: Page) {
  const errors: string[] = []
  const statuses: number[] = []; const failures: string[] = []
  page.on('pageerror', error => errors.push(error.name))
  page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push('console error') })
  page.on('response', response => { if (response.status() >= 400) statuses.push(response.status()) })
  page.on('requestfailed', request => failures.push(request.failure()?.errorText ?? 'failed'))
  return { errors, statuses, failures }
}
