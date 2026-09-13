import { test, expect, type Page } from '@playwright/test'
import { liveServer, mockWorkspace, scorecard, scoreUrl, round } from './returnLoadingSupport'
import { completion, tournament } from '../src/features/tournaments/lifecycle/__tests__/fixtures'
import type { ScoringScorecard } from '../src/api/scorecards'
import type { Round } from '../src/api/types'

async function layout(page: Page, name: string) {
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 800 })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const button of await page.locator('.scorecard-holes button').all()) {
      const box = await button.boundingBox()
      expect(box?.height).toBeGreaterThanOrEqual(44)
      await button.click({ trial: true })
      const badge = button.locator('.hole-handicap')
      if (await badge.count()) {
        const bounds = await badge.boundingBox()
        expect(bounds?.x).toBeGreaterThanOrEqual(0)
        expect((bounds?.x ?? 0) + (bounds?.width ?? 0)).toBeLessThanOrEqual(width)
      }
    }
    await page.evaluate(() => window.scrollTo(0, 0))
    await page.screenshot({ path: `/tmp/golf-handicap-${name}-${width}.png`, fullPage: true })
  }
}

test('summary displays server strokes for unscored, scored, zero and team cards', async ({ page }) => {
  const live = await liveServer()
  const errors: string[] = []
  page.on('pageerror', error => errors.push(error.message))
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()) })
  page.on('requestfailed', request => {
    if (!request.url().endsWith('/live') && request.failure()?.errorText !== 'net::ERR_ABORTED') errors.push(request.url())
  })
  try {
    await mockWorkspace(page, live.url)
    let card: ScoringScorecard = { ...scorecard(), playing_handicap: 20,
      holes: scorecard().holes.map((hole, i) => ({ ...hole, stroke_index: 18 - i, handicap_strokes: i >= 16 ? 2 : 1 })) }
    await page.route(url => url.pathname === `/api/tournaments/${tournament.id}/rounds`, route => route.fulfill({ json: [{
      ...round, number_of_holes: card.number_of_holes, scoring_format: card.owner.type === 'team' ? 'two_player_foursomes' : 'individual_stroke_play',
      handicap_allowance_percent: card.owner.type === 'team' ? 50 : 100,
    } satisfies Round] }))
    await page.route('**/completion-validation', route => route.fulfill({ json: {
      ...completion(), ready_to_complete: false, ready_to_lock: false,
      owners: [{ owner: card.owner, owner_name: 'Spiller eller lag med et veldig langt navn i oppsummeringen',
        holes_scored: card.holes_scored, required_holes: card.number_of_holes, complete: card.complete, confirmed: false }],
      issues: [...completion().issues, { code: 'incomplete_scorecards', message: 'Ikke ferdig' },
        { code: 'unconfirmed_scorecards', message: 'Ikke bekreftet' }],
    } }))
    await page.route('**/score-access', route => route.fulfill({ json: { round_id: round.id, writable_owners: [card.owner] } }))
    await page.route('**/scoring', route => route.fulfill({ json: card }))
    await page.goto(scoreUrl.replace('view=hole', 'view=summary'))
    await expect(page.getByRole('heading', { name: 'Oppsummering', exact: true })).toBeVisible()
    await expect(page.getByRole('img', { name: '2 ekstra slag på hull 17', exact: true })).toHaveText('+2')
    await expect(page.locator('.hole-handicap')).toHaveCount(18)
    await expect(page.locator('.scorecard-holes').getByText('Netto –', { exact: true })).toHaveCount(18)
    await layout(page, 'unscored')
    await page.getByRole('button', { name: /Hull 17 Par/ }).click()
    await expect(page.locator('#current-hole-heading')).toHaveText('17')
    card = { ...card, gross_total: 5, net_total: 3, holes_scored: 1,
      holes: card.holes.map(hole => hole.hole_number !== 17 ? hole : { ...hole, net_strokes: 3,
        score: { id: '00000000-0000-0000-0000-000000000099', round_id: round.id, hole_id: hole.hole_id,
          owner: card.owner, gross_strokes: 5, submitted_by: card.owner.id, submitted_at: '2026-09-13T12:00:00Z', updated_at: '2026-09-13T12:00:00Z' } }) }
    await page.goto(scoreUrl.replace('view=hole', 'view=summary'))
    await expect(page.getByRole('button', { name: /Hull 17 Par/ })).toContainText('Netto 3')
    await expect(page.getByRole('img', { name: '2 ekstra slag på hull 17', exact: true })).toHaveText('+2')
    await layout(page, 'scored')
    card = { ...scorecard() }
    await page.reload()
    await expect(page.getByRole('heading', { name: 'Oppsummering', exact: true })).toBeVisible()
    await expect(page.locator('.hole-handicap')).toHaveCount(0)
    card = { ...scorecard(), owner: { type: 'team', id: card.owner.id }, number_of_holes: 9, playing_handicap: -11,
      holes: scorecard().holes.slice(0, 9).map((hole, i) => ({ ...hole, stroke_index: 9 - i, handicap_strokes: i < 2 ? -2 : -1 })) }
    await page.goto(scoreUrl.replace('owner_type=player', 'owner_type=team').replace('view=hole', 'view=summary'))
    await expect(page.getByText('Slag som gis tilbake fra spillehandicap')).toBeVisible()
    await expect(page.getByRole('img', { name: '2 slag gis tilbake på hull 1', exact: true })).toHaveText('−2')
    await expect(page.locator('.hole-handicap')).toHaveCount(9)
    await layout(page, 'team-negative-nine')
    expect(errors).toEqual([])
  } finally {
    await live.close()
  }
})

test('restricted summary retains full-round allocation only on the visible nine', async ({ page }) => {
  const live = await liveServer()
  try {
    const state = await mockWorkspace(page, live.url)
    state.readOnly = true; state.restricted = true
    const card = scorecard()
    await page.route('**/scorecards/player/*', route => route.fulfill({ json: {
      projection: 'read', round_id: round.id, owner: card.owner, number_of_holes: 18, visible_hole_count: 9,
      playing_handicap: 18, gross_total: 0, net_total: 0, holes_scored: 0,
      complete: null, confirmed: null, confirmed_at: null, visibility: { mode: 'front_nine' },
      holes: card.holes.slice(0, 9).map(hole => ({ ...hole, handicap_strokes: 1 })),
    } }))
    await page.goto(scoreUrl.replace('view=hole', 'view=summary'))
    await expect(page.getByText('Hull 10–18 er skjult til administratoren frigir finalens bakni.')).toBeVisible()
    await expect(page.locator('.hole-handicap')).toHaveCount(9)
    await expect(page.getByRole('img', { name: '1 ekstra slag på hull 9', exact: true })).toHaveText('+1')
    await expect(page.getByRole('button', { name: /Hull 10 Par/ })).toHaveCount(0)
    await layout(page, 'restricted')
  } finally { await live.close() }
})
