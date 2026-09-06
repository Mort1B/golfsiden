import { expect, type APIRequestContext, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeCompletionValidation, decodeScoringScorecard, ownerTypeForFormat, type ScoreOwner } from '../src/api/scorecards'
import { decodeTournamentLeaderboard, decodeRoundLeaderboard } from '../src/api/leaderboards'
import type { LeaderboardMetric, Round, TournamentLeaderboard } from '../src/api/types'

export async function login(page: Page, username = 'admin') {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill(username)
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
  return decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
}
export async function mutate(api: APIRequestContext, path: string, csrf: string, data: unknown = {}, method = 'POST') {
  const response = await api.fetch(path, { method, data, headers: { 'x-csrf-token': csrf } })
  expect(response.ok(), `${method} ${path}: ${response.status()}`).toBe(true)
}
export async function progress(api: APIRequestContext, round: Round) {
  return decodeCompletionValidation(await (await api.get(`/api/rounds/${round.id}/completion-validation`)).json(), round.id, ownerTypeForFormat(round.scoring_format))
}
export async function scorecard(api: APIRequestContext, round: Round, owner: ScoreOwner) {
  return decodeScoringScorecard(await (await api.get(`/api/rounds/${round.id}/scorecards/${owner.type}/${owner.id}/scoring`)).json(), round.id, owner)
}
export async function tournamentBoard(api: APIRequestContext, id: string, metric: LeaderboardMetric) {
  return decodeTournamentLeaderboard(await (await api.get(`/api/tournaments/${id}/leaderboards/${metric}`)).json(), id, metric)
}
export async function roundBoard(api: APIRequestContext, round: Round, metric: LeaderboardMetric) {
  return decodeRoundLeaderboard(await (await api.get(`/api/rounds/${round.id}/leaderboards/${metric}`)).json(), round.id, round.tournament_id, metric)
}
export async function assertRendered(page: Page, board: TournamentLeaderboard) {
  for (const entry of board.entries) {
    const row = page.locator(`.leaderboard-row-link[href*="/players/${entry.player_id}?"]`)
    const total = board.metric === 'gross' ? entry.gross_total : entry.net_total
    if (entry.contributions.some(item => item.counted)) {
      await expect(row.locator('.leaderboard-score span')).toContainText(`${total} ${board.metric === 'gross' ? 'brutto' : 'netto'}`)
    } else await expect(row.locator('.leaderboard-score strong')).toHaveText('–')
  }
  await expect(page.locator('.leaderboard-page [role="alert"]')).toHaveCount(0)
}
export function nextBoard(page: Page, id: string, metric: LeaderboardMetric) {
  return page.waitForResponse(response => new URL(response.url()).pathname === `/api/tournaments/${id}/leaderboards/${metric}` && response.ok())
}
export async function fillAndConfirm(api: APIRequestContext, round: Round, csrf: string, strokes = 4) {
  for (const { owner } of (await progress(api, round)).owners) {
    const card = await scorecard(api, round, owner)
    for (const hole of card.holes) {
      await mutate(api, `/api/rounds/${round.id}/scores`, csrf, { owner, hole_id: hole.hole_id, gross_strokes: strokes }, 'PUT')
    }
    await mutate(api, `/api/rounds/${round.id}/scorecards/${owner.type}/${owner.id}/confirm`, csrf)
  }
}
export async function layout(page: Page, state: string) {
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 900 })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    await page.screenshot({ path: `/tmp/golf-leaderboard-live-${state}-${width}.png`, fullPage: true })
  }
}
