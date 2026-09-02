import { QueryClient } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { leaderboardKeys } from '../../api/leaderboards'
import { scoringKeys } from '../../api/scorecards'
import { invalidateScorecard, invalidateScoreDependents } from './queries'

describe('score dependent invalidation', () => {
  it('invalidates the canonical keys shared by leaderboard drilldowns', async () => {
    const queryClient = new QueryClient()
    const roundKey = leaderboardKeys.round('user', 'round', 'net')
    const tournamentKey = leaderboardKeys.tournament('user', 'tournament', 'net')
    queryClient.setQueryData(roundKey, { entries: [] })
    queryClient.setQueryData(tournamentKey, { entries: [] })

    await invalidateScoreDependents(queryClient, 'user', 'round', 'tournament')

    expect(queryClient.getQueryState(roundKey)?.isInvalidated).toBe(true)
    expect(queryClient.getQueryState(tournamentKey)?.isInvalidated).toBe(true)
  })

  it('invalidates the canonical actor-free card key used by direct reads', async () => {
    const queryClient = new QueryClient()
    const owner = { type: 'team' as const, id: 'team' }
    const readKey = scoringKeys.read('user', 'round', owner)
    queryClient.setQueryData(readKey, { projection: 'read' })

    await invalidateScorecard(queryClient, 'user', 'round', owner)

    expect(queryClient.getQueryState(readKey)?.isInvalidated).toBe(true)
  })
})
