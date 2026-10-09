import { expect, test } from '@playwright/test'
import { fantasyLayouts } from './fantasySupport'
import { at, browserEvidence, claimManager, releaseFixture, scoreTeam, selectFour } from './fantasyReleaseSupport'

test.skip(process.env.GOLF_FANTASY_BROWSER !== '1', 'Requires disposable local API and Chrome.')
test('nine teams retain changing partners, picks, captains and two carry-forward generations', async ({ page, browser }) => {
  const evidence = browserEvidence(page)
  const f = await releaseFixture(page, ['team_scramble', 'two_player_foursomes', 'team_scramble'], Array.from({ length: 18 }, () => 0))
  const carried = await claimManager(f, browser, 1), missed = await claimManager(f, browser, 17)
  try {
    const orders = [Array.from({ length: 18 }, (_, i) => i), [0, 2, 1, 3, 4, 6, 5, 7, 8, 10, 9, 11, 12, 14, 13, 15, 16, 17], [0, 3, 1, 4, 2, 5, 6, 9, 7, 10, 8, 11, 12, 15, 13, 16, 14, 17]]
    const teams = []
    for (let r = 0; r < 3; r++) teams.push(await f.configure(r, at(orders, r), true))
    expect((await carried.page.request.put(`${f.base}/rounds/${at(f.rounds, 0).id}/lineup`, { headers: { 'x-csrf-token': carried.auth.csrf_token }, data: {
      request_id: crypto.randomUUID(), expected_revision: 0, picks: f.players.slice(0, 4), captain: at(f.players, 1),
    } })).ok()).toBe(true)
    await selectFour(f, 0, [0, 1, 2, 3], 0); await f.start()
    const placement = [10, 8, 6, 5, 4, 3, 2, 1, 0], base = [10, 7, 4, 2, -1, -2, -3, -4, -5]
    const adminTotals = [44, 14, 19], carryTotals = [44, 41, 38]
    for (let r = 0; r < 3; r++) {
      if (r === 1) await selectFour(f, r, [0, 4, 8, 12], 4)
      await f.mutate(`/api/rounds/${at(f.rounds, r).id}/open`)
      for (const [i, team] of at(teams, r).entries()) await scoreTeam(f, r, team.id, 4 + i)
      await f.mutate(`/api/rounds/${at(f.rounds, r).id}/complete`)
      const board = await f.board(r)
      expect(board.golfers).toHaveLength(18)
      for (const [i, team] of at(teams, r).entries()) for (const member of team.members) {
        const golfer = board.golfers.find(g => g.player_id === member.player_id)
        expect(golfer?.placement_points).toBe(at(placement, i)); expect(golfer?.points).toEqual({ state: 'settled', total: at(base, i) }); expect(golfer?.team_id).toBe(team.id)
      }
      const admin = board.managers.find(m => m.user_id === f.auth.user_id), carry = board.managers.find(m => m.user_id === carried.auth.user_id), absent = board.managers.find(m => m.user_id === missed.auth.user_id)
      expect(admin?.points).toEqual({ state: 'settled', total: at(adminTotals, r) }); expect(carry?.points).toEqual({ state: 'settled', total: at(carryTotals, r) })
      expect(carry?.lineup?.captain).toBe(at(f.players, 1)); expect(carry?.lineup?.picks).toEqual(expect.arrayContaining(f.players.slice(0, 4)))
      expect(carry?.lineup?.origin).toBe(r === 0 ? 'submitted' : 'carried_forward'); expect(carry?.lineup?.source_round).toBe(r === 0 ? null : at(f.rounds, r - 1).id)
      expect(admin?.lineup?.captain).toBe(at(f.players, r === 0 ? 0 : 4))
      expect(absent?.selection_state).toBe('missed'); expect(absent?.lineup).toBeNull(); expect(absent?.points).toEqual({ state: 'settled', total: 0 })
    }
    const total = await f.totals()
    expect(total.managers.find(m => m.id === f.auth.user_id)?.points).toEqual({ state: 'settled', total: 77 })
    expect(total.managers.find(m => m.id === carried.auth.user_id)?.points).toEqual({ state: 'settled', total: 123 })
    expect(total.golfers.find(g => g.id === at(f.players, 0))?.points).toEqual({ state: 'settled', total: 30 })
    expect(total.golfers.find(g => g.id === at(f.players, 1))?.points).toEqual({ state: 'settled', total: 24 })
    expect(total.golfers.find(g => g.id === at(f.players, 17))?.points).toEqual({ state: 'settled', total: -15 })
    await page.goto(f.url); await page.getByRole('button', { name: 'Poengtavler', exact: true }).click()
    await expect(page.locator('.fantasy-standings > li')).toHaveCount(3)
    await expect(page.locator('.fantasy-standings button').filter({ hasText: at(f.names, 1) })).toContainText('123 p · avgjort')
    await page.locator('.fantasy-standings button').filter({ hasText: at(f.names, 1) }).click()
    await expect(page.locator('.fantasy-detail').getByText('Gjenbrukt fra tidligere låst runde', { exact: true })).toHaveCount(2)
    await expect(page.locator('.fantasy-contributions li')).toHaveCount(12)
    await fantasyLayouts(page, 'release-nine-teams-managers')
    await page.getByRole('button', { name: 'Spillerpoeng', exact: true }).click()
    await expect(page.locator('.fantasy-standings > li')).toHaveCount(18)
    await expect(page.locator('.fantasy-standings button').filter({ hasText: at(f.names, 17) })).toContainText('-15 p · avgjort')
    await page.locator('.fantasy-standings button').filter({ hasText: at(f.names, 0) }).click()
    await expect(page.locator('.fantasy-detail')).toContainText('30 p · avgjort · uten kapteinmultiplikator')
    await expect(page.locator('.fantasy-detail').getByText('Plasseringspoeng: 10', { exact: true })).toHaveCount(3)
    await fantasyLayouts(page, 'release-nine-teams-golfers')
    await page.getByLabel('Vis poeng for').selectOption(at(f.rounds, 1).id)
    await expect(page.locator('.fantasy-standings > li')).toHaveCount(18)
    await expect(page.locator('.fantasy-standings button').filter({ hasText: at(f.names, 1) })).toContainText('7 p · avgjort')
    await evidence()
  } finally { await carried.context.close(); await missed.context.close() }
})
