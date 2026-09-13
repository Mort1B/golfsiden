import { expect, test } from '@playwright/test'
import { stablefordFixture } from './stablefordSupport'
import { decodeTournamentLeaderboard, validateTournamentLeaderboardRounds } from '../src/api/leaderboards'
import { decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { offlineEvents, offlineLayout } from './offlineSupport'

test.skip(process.env.GOLF_STABLEFORD_BROWSER !== '1', 'Requires disposable local API and Chrome')
for (const mixed of [false, true]) {
  test(`Draft Stableford selects overall equivalents with ${mixed ? 'completed stroke results' : 'no started rounds'}`, async ({ page }) => {
    const events = offlineEvents(page)
    await stablefordFixture(page, undefined, mixed, async tournamentId => {
      const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${tournamentId}/rounds`)).json(), tournamentId)
      expect(rounds.find(round => round.scoring_format === 'individual_stableford')?.status).toBe('draft')
      for (const metric of ['gross', 'net'] as const) {
        const response = await page.request.get(`/api/tournaments/${tournamentId}/leaderboards/${metric}`)
        expect(response.status()).toBe(200)
        const board = validateTournamentLeaderboardRounds(decodeTournamentLeaderboard(await response.json(), tournamentId, metric), rounds)
        expect(board.entries).toHaveLength(1)
        expect(board.entries[0]?.value).toMatchObject({ type: 'overall_equivalent', version: 1 })
        expect(board.entries[0]?.contributions).toHaveLength(mixed ? 1 : 0)
        expect(board.included_round_ids).toHaveLength(mixed ? 1 : 0)
      }
      await page.goto(`/leaderboard?tournament=${tournamentId}&metric=net`)
      await page.getByRole('button', { name: 'Turnering', exact: true }).click()
      await expect(page.getByText(/Sammenlagtekvivalent/).first()).toBeVisible()
      await offlineLayout(page, `stableford-draft-${mixed ? 'mixed' : 'empty'}`)
      expect(events.errors).toEqual([])
      expect(events.statuses.filter(status => status !== 401)).toEqual([])
    })
  })
}
